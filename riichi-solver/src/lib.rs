//! Single-player EV solver: tenpai probability, win probability, and expected
//! score per-turn vectors for a hand (closed part + fixed melds), via a
//! memoized state graph and backward DP over turns.
//!
//! Clean-room implementation from nekobean/mahjong-cpp's algorithm docs
//! (`docs/expected_score_calculation.md`, `docs/uradora_expected_value.md`)
//! and tomohxx's algorithm book. No GPL source was consulted. Golden test
//! values come from running the mahjong-cpp binary.
//!
//! Model (matching the reference semantics):
//! - Single player, tsumo-only wins; opponents unmodeled; discards return to
//!   the wall; wall total shrinks by 1 per turn.
//! - Turn `t` counts draws; boundary at `t_max`. The obs-path convention is
//!   `t_max = actual tsumos left, capped at` [`MAX_TSUMOS_LEFT`].
//! - Melds are fixed input (no mid-search calls); their tiles are visible
//!   (out of the wall) and count for dora/uradora and scoring.
//! - Auto-riichi: a menzen tenpai hand (no melds, or ankan only) is treated
//!   as having declared riichi (riichi + menzen tsumo yaku, uradora EV).
//! - Wins happen on draw edges and require yaku: a yakuless completion
//!   (open keishiki tenpai) counts as tenpai but never as a win.
//! - Expansion: shanten-advancing draws and min-shanten discards only
//!   (tegawari and shanten-back off — the production obs-path config).
//! - Scoring reuses the `riichi` crate's yaku/fu scorer; uradora is an exact
//!   combinatorial expectation over the remaining wall.
//!
//! Production entry: [`Solver::analyze`] — preconditions, then the
//! [`SHANTEN_GATE`] decides full DP vs ukeire-only simple mode.

use riichi::engine::agari::{agari_candidates, AgariInput};
use riichi::engine::distribute_points;
use riichi::model::{DoraHits, RoundId, Scoring};
use riichi::rules::Ruleset;
use riichi_decomp::{Decomposer, ShantenLut, WaitSet};
use riichi_elements::prelude::*;
use rustc_hash::FxHashMap as HashMap;

pub const T_MAX: usize = 18;
/// Obs-path horizon convention: actual tsumos left, capped at 17. Callers
/// set `SolverConfig::t_max = tsumos_left.min(MAX_TSUMOS_LEFT)`.
pub const MAX_TSUMOS_LEFT: usize = 17;
/// Full tenpai/win/EV tables only at or below this root shanten; beyond it
/// [`Solver::analyze`] returns ukeire-only simple stats.
pub const SHANTEN_GATE: i8 = 3;
const N_KINDS: usize = 37; // 34 normal + 3 red fives
const RED_BASE: usize = 34;

/// Per-kind copies in a full set: 3 for normal fives, 1 for reds, 4 otherwise.
fn kind_copies(k: usize) -> u8 {
    match k {
        4 | 13 | 22 => 3,
        RED_BASE..=36 => 1,
        _ => 4,
    }
}

/// 37-kind index folded to its 34-kind (reds to their fives).
fn fold_kind(k: usize) -> usize {
    match k {
        34 => 4,
        35 => 13,
        36 => 22,
        _ => k,
    }
}

/// Fold a 37-slot count array to 34 kinds (reds into their fives).
fn fold34(hand: &[u8; N_KINDS]) -> TileSet34 {
    let mut a = [0u8; 34];
    a.copy_from_slice(&hand[..34]);
    a[4] += hand[34];
    a[13] += hand[35];
    a[22] += hand[36];
    TileSet34(a)
}

fn to_tileset37(hand: &[u8; N_KINDS]) -> TileSet37 {
    TileSet37(*hand)
}

/// Pack a 37-slot hand into a u128 key (3 bits per slot).
fn key(hand: &[u8; N_KINDS]) -> u128 {
    let mut k = 0u128;
    for &c in hand.iter() {
        k = (k << 3) | c as u128;
    }
    k
}

pub struct SolverConfig {
    pub t_max: usize,
    /// Total wall tiles (unseen from the root hand's perspective).
    pub sum: u32,
    pub dora_indicators: Vec<Tile>,
    /// Round context for scoring. Defaults: East-1, we are the non-dealer South seat.
    pub round_id: RoundId,
    pub seat: Player,
    pub enable_uradora: bool,
    /// Fixed melds (never changed mid-search). Riichi/uradora apply only
    /// when the hand is menzen (no melds, or ankan only).
    pub melds: Vec<Meld>,
}

impl SolverConfig {
    /// Reference-parity defaults for a given root hand size (closed tiles).
    pub fn new(root_tiles: u32, dora_indicators: Vec<Tile>) -> Self {
        let sum = 136 - root_tiles - dora_indicators.len() as u32;
        SolverConfig {
            t_max: T_MAX,
            sum,
            dora_indicators,
            round_id: RoundId { kyoku: 0, honba: 0 },
            seat: Player::new(1),
            enable_uradora: true,
            melds: Vec::new(),
        }
    }

    /// Attach melds; their tiles are visible, so they leave the wall.
    pub fn with_melds(mut self, melds: Vec<Meld>) -> Self {
        for m in &melds {
            self.sum -= m.to_tiles().len() as u32;
        }
        self.melds = melds;
        self
    }
}

/// Result for one root discard candidate (or the whole hand for a 13-tile root).
pub struct Stat {
    /// Discarded tile kind (37-encoding), or None for a 13-tile root.
    pub tile: Option<u8>,
    pub shanten: i8,
    /// Indexed by turn 0..=t_max; entries 1..=t_max are meaningful.
    pub tenpai_prob: Vec<f64>,
    pub win_prob: Vec<f64>,
    pub exp_score: Vec<f64>,
    /// Necessary (shanten-advancing) tiles, folded to 34 kinds: (kind, wall copies).
    pub necessary: Vec<(u8, u8)>,
}

struct DrawEdge {
    kind: u8,
    w: u8,
    to: usize,
    /// Expected winner gain if this draw completes a win (0 otherwise).
    score: f64,
    /// Back edge to the 14-tile root (not a shanten-advancing draw).
    synthetic: bool,
}

struct Node13 {
    hand: [u8; N_KINDS],
    shanten: i8,
    tenpai: bool,
    draws: Vec<DrawEdge>,
}

struct Node14 {
    win: bool,
    tenpai: bool,
    /// Min-shanten discards: (discarded kind, resulting 13-tile node).
    /// These are also the candidate discards reported for a 14-tile root.
    discards: Vec<(u8, usize)>,
    /// Undo-discards: reverses of incoming advancing draw edges (discard the
    /// drawn tile family back, returning to a worse-shanten 13-tile node).
    /// The reference DP reads its bipartite edge set in both directions, so
    /// these are legal discard choices — occasionally a profitable reshape —
    /// but never root candidates.
    undo: Vec<usize>,
}

pub struct Solver {
    decomposer: Decomposer<'static>,
    ruleset: Ruleset,
    lut: &'static ShantenLut,
}

struct Build<'a> {
    cfg: &'a SolverConfig,
    ind34: [u8; 34],
    /// Meld tiles per 37-kind (visible: out of the wall).
    meld37: [u8; N_KINDS],
    /// Meld tiles folded to 34 kinds (dora/uradora hits include melds).
    meld34: [u8; 34],
    /// Dora hits inside melds (per current indicators) and red fives there.
    meld_dora: u8,
    meld_aka: u8,
    /// No melds, or ankan only: riichi/uradora eligible.
    menzen: bool,
    n13: Vec<Node13>,
    n14: Vec<Node14>,
    memo13: HashMap<u128, usize>,
    memo14: HashMap<u128, usize>,
    /// Key of a 13-tile root hand. Riichi is declared on a tenpai-keeping
    /// discard, so a tenpai 13-tile *input* has not declared riichi: its win
    /// edges score without riichi/uradora. Every other tenpai 13-node is
    /// reached via a discard and scores as riichi. (No collision is possible:
    /// a tenpai root's graph contains no other 13-node with the same hand.)
    root13_key: Option<u128>,
}

impl<'a> Build<'a> {
    fn new(cfg: &'a SolverConfig, root_hand: &[u8; N_KINDS]) -> Build<'a> {
        let mut ind34 = [0u8; 34];
        for t in &cfg.dora_indicators {
            ind34[t.normal_encoding() as usize] += 1;
        }
        let mut meld37 = [0u8; N_KINDS];
        let mut meld_aka = 0u8;
        for m in &cfg.melds {
            for t in m.to_tiles() {
                meld37[t.encoding() as usize] += 1;
                if t.is_red() {
                    meld_aka += 1;
                }
            }
        }
        let mut meld_dora = 0u8;
        for t in &cfg.dora_indicators {
            let d = t.indicated_dora().normal_encoding() as usize;
            meld_dora += meld37[d];
            // Red fives sit in slots 34..37; fold them onto their five.
            if d == 4 {
                meld_dora += meld37[34];
            } else if d == 13 {
                meld_dora += meld37[35];
            } else if d == 22 {
                meld_dora += meld37[36];
            }
        }
        let mut meld34 = [0u8; 34];
        meld34.copy_from_slice(&meld37[..34]);
        meld34[4] += meld37[34];
        meld34[13] += meld37[35];
        meld34[22] += meld37[36];

        let n_tiles: u32 = fold34(root_hand).0.iter().map(|&c| c as u32).sum();
        Build {
            cfg,
            ind34,
            meld37,
            meld34,
            meld_dora,
            meld_aka,
            menzen: cfg.melds.iter().all(|m| m.is_closed()),
            n13: Vec::new(),
            n14: Vec::new(),
            memo13: HashMap::default(),
            memo14: HashMap::default(),
            root13_key: (n_tiles % 3 == 1).then(|| key(root_hand)),
        }
    }
}

impl Solver {
    pub fn new() -> Self {
        Solver {
            decomposer: Decomposer::new(),
            ruleset: Ruleset::default(),
            lut: ShantenLut::get(),
        }
    }

    /// Solve a root hand (13 or 14 tiles, closed). Returns per-candidate stats
    /// and the number of graph vertices searched.
    pub fn solve(&mut self, hand: &TileSet37, cfg: &SolverConfig) -> (Vec<Stat>, usize) {
        let root_hand = hand.0;
        let n_tiles: u32 = fold34(&root_hand).0.iter().map(|&c| c as u32).sum();
        let mut b = Build::new(cfg, &root_hand);

        let root_stats: Vec<(Option<u8>, usize)> = if n_tiles % 3 == 1 {
            let id = self.build13(&mut b, root_hand);
            vec![(None, id)]
        } else {
            // 14 tiles: the root is itself a graph node; its min-shanten
            // discards are the candidates.
            let root_id = self.build14(&mut b, root_hand);
            b.n14[root_id]
                .discards
                .clone()
                .into_iter()
                .map(|(k, id)| (Some(k), id))
                .collect()
        };

        let (v13, _v14) = self.run_dp(&b);

        let stats = root_stats
            .into_iter()
            .map(|(tile, id)| {
                let node = &b.n13[id];
                let mut necessary: Vec<(u8, u8)> = Vec::new();
                for e in node.draws.iter().filter(|e| !e.synthetic) {
                    let nk = if (e.kind as usize) >= RED_BASE {
                        match e.kind as usize {
                            34 => 4u8,
                            35 => 13,
                            _ => 22,
                        }
                    } else {
                        e.kind
                    };
                    match necessary.iter_mut().find(|(k, _)| *k == nk) {
                        Some((_, c)) => *c += e.w,
                        None => necessary.push((nk, e.w)),
                    }
                }
                necessary.sort_unstable();
                let t_max = cfg.t_max;
                let mut tenpai_prob = vec![0.0; t_max + 1];
                let mut win_prob = vec![0.0; t_max + 1];
                let mut exp_score = vec![0.0; t_max + 1];
                for t in 1..=t_max {
                    let v = &v13[id * (t_max + 1) + t];
                    tenpai_prob[t] = v[0];
                    win_prob[t] = v[1];
                    exp_score[t] = v[2];
                }
                Stat {
                    tile,
                    shanten: node.shanten,
                    tenpai_prob,
                    win_prob,
                    exp_score,
                    necessary,
                }
            })
            .collect();

        (stats, b.n13.len() + b.n14.len())
    }

    /// Wall copies of kind `k` unseen from `hand`'s perspective.
    fn wall_count(b: &Build, hand: &[u8; N_KINDS], k: usize) -> u8 {
        let ind = if k < 34 { b.ind34[k] } else { 0 };
        kind_copies(k)
            .saturating_sub(hand[k])
            .saturating_sub(ind)
            .saturating_sub(b.meld37[k])
    }

    fn build13(&mut self, b: &mut Build, hand: [u8; N_KINDS]) -> usize {
        let hk = key(&hand);
        if let Some(&id) = b.memo13.get(&hk) {
            return id;
        }
        let id = b.n13.len();
        b.n13.push(Node13 {
            hand,
            shanten: 0,
            tenpai: false,
            draws: Vec::new(),
        });
        b.memo13.insert(hk, id);

        let (s13, advancing) = self.lut.analyze_13(&fold34(&hand));
        let tenpai = s13 == 0;
        // Wait set of the 13-tile hand, needed to score winning draws.
        let wait_set = if tenpai {
            Some(WaitSet::from_tile_set(&mut self.decomposer, &fold34(&hand)))
        } else {
            None
        };

        let mut draws = Vec::new();
        for k in 0..N_KINDS {
            if advancing & (1 << fold_kind(k)) == 0 {
                continue;
            }
            let w = Self::wall_count(b, &hand, k);
            if w == 0 {
                continue;
            }
            let mut h2 = hand;
            h2[k] += 1;
            let score = if tenpai {
                // Riichi requires menzen; the 13-tile root additionally has
                // not declared yet (declaration happens on a discard).
                let riichi = b.menzen && b.root13_key != Some(hk);
                self.score_win(b, &hand, wait_set.as_ref().unwrap(), k, riichi)
            } else {
                0.0
            };
            let to = self.build14(b, h2);
            b.n14[to].undo.push(id);
            draws.push(DrawEdge {
                kind: k as u8,
                w,
                to,
                score,
                synthetic: false,
            });
        }
        b.n13[id].shanten = s13;
        b.n13[id].tenpai = tenpai;
        b.n13[id].draws = draws;
        id
    }

    fn build14(&mut self, b: &mut Build, hand: [u8; N_KINDS]) -> usize {
        let hk = key(&hand);
        if let Some(&id) = b.memo14.get(&hk) {
            return id;
        }
        let id = b.n14.len();
        b.n14.push(Node14 {
            win: false,
            tenpai: false,
            discards: Vec::new(),
            undo: Vec::new(),
        });
        b.memo14.insert(hk, id);

        let (s14, keep) = self.lut.analyze_14(&fold34(&hand));
        // Win nodes are terminal: the hand is closed and tenpai, so riichi is
        // locked — continuing can never beat taking the win (same waits, same
        // score, fewer draws left). Expanding their discards would also let
        // the graph wander between tenpai hands unboundedly.
        let discards: Vec<(u8, usize)> = if s14 == -1 {
            Vec::new()
        } else {
            // Min-shanten discards only (shanten-back off): a 14-tile hand's
            // shanten equals its best discard's, so these are exactly the
            // keep-shanten discards.
            let mut cand: Vec<(u8, [u8; N_KINDS])> = Vec::new();
            for k in 0..N_KINDS {
                if hand[k] == 0 || keep & (1 << fold_kind(k)) == 0 {
                    continue;
                }
                let mut h2 = hand;
                h2[k] -= 1;
                cand.push((k as u8, h2));
            }
            // Each discard edge also gets a reverse draw edge (redrawing the
            // discarded tile back into this 14-tile hand), letting the DP
            // revisit the discard choice later. This adds edges between
            // existing vertices only, and never duplicates an advancing
            // draw: a min-shanten discard means parent and child shanten are
            // equal, so the redraw is shanten-neutral.
            cand.into_iter()
                .map(|(k, h)| {
                    let child = self.build13(b, h);
                    let w = Self::wall_count(b, &b.n13[child].hand.clone(), k as usize);
                    if w > 0 {
                        b.n13[child].draws.push(DrawEdge {
                            kind: k,
                            w,
                            to: id,
                            score: 0.0,
                            synthetic: true,
                        });
                    }
                    (k, child)
                })
                .collect()
        };
        let n = &mut b.n14[id];
        n.win = s14 == -1;
        n.tenpai = s14 <= 0;
        n.discards = discards;
        id
    }

    /// Expected winner gain for drawing `draw_kind` into tenpai `hand13`,
    /// including exact uradora EV when riichi.
    fn score_win(
        &mut self,
        b: &Build,
        hand13: &[u8; N_KINDS],
        wait_set: &WaitSet,
        draw_kind: usize,
        riichi: bool,
    ) -> f64 {
        let win_tile = Tile::from_encoding(draw_kind as u8).unwrap();
        if !wait_set.waiting_tiles.has(win_tile.to_normal()) {
            // Advancing draw on a tenpai hand is always a completing draw;
            // guard anyway (e.g. structurally-tenpai edge cases).
            return 0.0;
        }
        let closed = to_tileset37(hand13);
        let input = AgariInput {
            round_id: b.cfg.round_id,
            winner: b.cfg.seat,
            closed_hand: &closed,
            riichi: riichi.then_some(riichi::model::Riichi {
                is_double: false,
                is_ippatsu: false,
            }),
            melds: &b.cfg.melds,
            wait_set,
            contributor: b.cfg.seat,
            incoming_is_kan: false,
            action_is_kan: false,
            winning_tile: win_tile,
            is_first_chance: false,
            is_last_draw: false,
        };
        let candidates = agari_candidates(&self.ruleset, &input);
        if candidates.is_empty() {
            return 0.0; // yakuless (unreachable for closed tsumo)
        }

        // All 14 tiles, folded, for dora counting; melds count too.
        let mut hand14 = *hand13;
        hand14[draw_kind] += 1;
        let all34 = fold34(&hand14);
        let dora: u8 = b
            .cfg
            .dora_indicators
            .iter()
            .map(|i| all34[i.indicated_dora()])
            .sum::<u8>()
            + b.meld_dora;
        let aka: u8 = hand14[34] + hand14[35] + hand14[36] + b.meld_aka;

        let gain = |ura: u8| -> f64 {
            let dh = DoraHits {
                dora,
                ura_dora: ura,
                aka_dora: aka,
            };
            let basic = candidates
                .iter()
                .map(|c| {
                    Scoring {
                        yakuman_total_value: c.scoring.yakuman_total_value,
                        yaku_total_value: c.scoring.yaku_total_value,
                        dora_hits: dh,
                        fu: c.scoring.fu,
                    }
                    .basic_points()
                })
                .max()
                .unwrap_or(0);
            distribute_points(
                &self.ruleset,
                b.cfg.round_id,
                false,
                b.cfg.seat,
                b.cfg.seat,
                basic,
            )[b.cfg.seat.to_usize()] as f64
        };

        if !riichi || !b.cfg.enable_uradora || b.cfg.dora_indicators.is_empty() {
            return gain(0);
        }

        // Exact uradora distribution: DP over the wall as seen from the win
        // state (docs/uradora_expected_value.md). Reds fold into their fives.
        let d = b.cfg.dora_indicators.len();
        let mut c_folded = [0u8; 34];
        for k in 0..N_KINDS {
            let w = Self::wall_count(b, &hand14, k);
            let nk = match k {
                34 => 4,
                35 => 13,
                36 => 22,
                _ => k,
            };
            c_folded[nk] += w;
        }
        let s_total: u32 = c_folded.iter().map(|&c| c as u32).sum();

        let binom = |n: u8, r: usize| -> f64 {
            let n = n as usize;
            if r > n {
                return 0.0;
            }
            let mut v = 1.0;
            for i in 0..r {
                v = v * (n - i) as f64 / (i + 1) as f64;
            }
            v
        };

        let mut dp = vec![[0.0f64; 13]; d + 1];
        dp[0][0] = 1.0;
        for i in 0..34 {
            let ci = c_folded[i];
            if ci == 0 {
                continue;
            }
            let ind_tile = Tile::from_encoding(i as u8).unwrap().indicated_dora();
            let gi =
                (all34[ind_tile] + b.meld34[ind_tile.normal_encoding() as usize]) as usize;
            let mut ndp = dp.clone();
            for x in 0..=d {
                for u in 0..13 {
                    if dp[x][u] == 0.0 {
                        continue;
                    }
                    let kmax = (ci as usize).min(d - x);
                    for kk in 1..=kmax {
                        let nu = (u + kk * gi).min(12);
                        ndp[x + kk][nu] += dp[x][u] * binom(ci, kk);
                    }
                }
            }
            dp = ndp;
        }
        let total = {
            let mut v = 1.0;
            for i in 0..d {
                v = v * (s_total as usize - i) as f64 / (i + 1) as f64;
            }
            v
        };
        (0..13).map(|u| dp[d][u] / total * gain(u as u8)).sum()
    }

    /// Backward DP over turns. Values per node: [tenpai_prob, win_prob, exp_score].
    fn run_dp(&self, b: &Build) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
        let t_max = b.cfg.t_max;
        let stride = t_max + 1;
        let mut v13 = vec![[0.0f64; 3]; b.n13.len() * stride];
        let mut v14 = vec![[0.0f64; 3]; b.n14.len() * stride];

        // Boundary t = t_max for 13-tile nodes.
        for (i, n) in b.n13.iter().enumerate() {
            v13[i * stride + t_max] = [if n.tenpai { 1.0 } else { 0.0 }, 0.0, 0.0];
        }
        // 14-tile nodes at t_max (clamps use same-turn 13-tile values).
        for (i, n) in b.n14.iter().enumerate() {
            v14[i * stride + t_max] = Self::node14_value(n, &v13, stride, t_max);
        }

        for t in (1..t_max).rev() {
            for (i, n) in b.n13.iter().enumerate() {
                let base = v13[i * stride + t + 1];
                let denom = (b.cfg.sum as f64) - t as f64;
                let mut v = base;
                for e in &n.draws {
                    let next = v14[e.to * stride + t + 1];
                    let p = e.w as f64 / denom;
                    // A win happens on the edge, and only with yaku (score >
                    // 0): the same complete hand reached via a different
                    // winning tile can be yakuless (open keishiki tenpai),
                    // which the reference counts as tenpai but never a win.
                    let win_val = if e.score > 0.0 { 1.0 } else { next[1] };
                    v[0] += p * (next[0] - base[0]);
                    v[1] += p * (win_val - base[1]);
                    v[2] += p * (e.score.max(next[2]) - base[2]);
                }
                v13[i * stride + t] = v;
            }
            for (i, n) in b.n14.iter().enumerate() {
                v14[i * stride + t] = Self::node14_value(n, &v13, stride, t);
            }
        }
        (v13, v14)
    }

    fn node14_value(
        n: &Node14,
        v13: &[[f64; 3]],
        stride: usize,
        t: usize,
    ) -> [f64; 3] {
        let mut best = [0.0f64; 3];
        for &(_, d) in &n.discards {
            let v = v13[d * stride + t];
            for j in 0..3 {
                best[j] = best[j].max(v[j]);
            }
        }
        for &d in &n.undo {
            let v = v13[d * stride + t];
            for j in 0..3 {
                best[j] = best[j].max(v[j]);
            }
        }
        [
            if n.tenpai { 1.0 } else { best[0] },
            // Complete nodes are terminal with no expansions, so this is 0
            // there: winning is accounted on the incoming edge (yaku only).
            best[1],
            best[2],
        ]
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

/// Ukeire-only result beyond the shanten gate.
pub struct SimpleStat {
    /// Discarded tile kind (37-encoding), or None for a 13-tile root.
    pub tile: Option<u8>,
    /// Shanten of the resulting 13-tile hand.
    pub shanten: i8,
    /// This discard raises shanten (14-tile roots only; shanten-down
    /// candidates only surface in simple mode — exploration is off in the
    /// full DP).
    pub shanten_down: bool,
    /// Advancing tiles of the resulting hand, folded to 34 kinds:
    /// (kind, wall copies).
    pub necessary: Vec<(u8, u8)>,
}

pub enum Analysis {
    /// Root shanten <= [`SHANTEN_GATE`]: full tenpai/win/EV tables.
    Full { stats: Vec<Stat>, searched: usize },
    /// Beyond the gate: ukeire only, no DP. `shanten` is the root's.
    Simple { shanten: i8, stats: Vec<SimpleStat> },
    /// Preconditions failed: no tsumos left, wall too small for the horizon,
    /// or the hand is already complete. Encoder-side fallback applies.
    Unavailable,
}

impl Solver {
    /// Production entry point: preconditions, then the shanten gate decides
    /// full DP vs ukeire-only simple mode. Works for action states (3N+2
    /// closed tiles: per-candidate stats) and reaction states (3N+1: a
    /// single current-hand stat).
    pub fn analyze(&mut self, hand: &TileSet37, cfg: &SolverConfig) -> Analysis {
        let folded = fold34(&hand.0);
        let n_tiles: u32 = folded.0.iter().map(|&c| c as u32).sum();
        if cfg.t_max == 0 || (cfg.sum as usize) <= cfg.t_max {
            return Analysis::Unavailable;
        }
        let shanten = if n_tiles % 3 == 2 {
            self.lut.analyze_14(&folded).0
        } else {
            self.lut.analyze_13(&folded).0
        };
        if shanten == -1 {
            return Analysis::Unavailable;
        }
        if shanten <= SHANTEN_GATE {
            let (stats, searched) = self.solve(hand, cfg);
            return Analysis::Full { stats, searched };
        }

        let b = Build::new(cfg, &hand.0);
        let stats = if n_tiles % 3 == 2 {
            let mut v = Vec::new();
            for k in 0..N_KINDS {
                if hand.0[k] == 0 {
                    continue;
                }
                let mut h2 = hand.0;
                h2[k] -= 1;
                let (s13, adv) = self.lut.analyze_13(&fold34(&h2));
                v.push(SimpleStat {
                    tile: Some(k as u8),
                    shanten: s13,
                    shanten_down: s13 > shanten,
                    necessary: necessary_from_mask(&b, &h2, adv),
                });
            }
            v
        } else {
            let (s13, adv) = self.lut.analyze_13(&folded);
            vec![SimpleStat {
                tile: None,
                shanten: s13,
                shanten_down: false,
                necessary: necessary_from_mask(&b, &hand.0, adv),
            }]
        };
        Analysis::Simple { shanten, stats }
    }
}

/// Folded (kind, wall copies) list for an advancing-tile mask of `hand`.
fn necessary_from_mask(b: &Build, hand: &[u8; N_KINDS], adv: u64) -> Vec<(u8, u8)> {
    let mut out: Vec<(u8, u8)> = Vec::new();
    for k in 0..N_KINDS {
        let fk = fold_kind(k);
        if adv & (1 << fk) == 0 {
            continue;
        }
        let w = Solver::wall_count(b, hand, k);
        if w == 0 {
            continue;
        }
        match out.iter_mut().find(|(kk, _)| *kk == fk as u8) {
            Some((_, c)) => *c += w,
            None => out.push((fk as u8, w)),
        }
    }
    out.sort_unstable();
    out
}

impl Solver {
    /// Debug: solve and dump the graph as JSON (spike diagnostics).
    #[doc(hidden)]
    pub fn graph_json(&mut self, hand: &TileSet37, cfg: &SolverConfig) -> String {
        let root_hand = hand.0;
        let n_tiles: u32 = fold34(&root_hand).0.iter().map(|&c| c as u32).sum();
        let mut b = Build::new(cfg, &root_hand);
        let root = if n_tiles % 3 == 1 {
            let id = self.build13(&mut b, root_hand);
            format!("{{\"type\":13,\"id\":{id}}}")
        } else {
            let id = self.build14(&mut b, root_hand);
            format!("{{\"type\":14,\"id\":{id}}}")
        };
        let mut out = String::new();
        out.push_str(&format!("{{\"root\":{root},\"sum\":{},\"n13\":[", cfg.sum));
        for (i, n) in b.n13.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let hand_arr: Vec<String> = n.hand.iter().map(|c| c.to_string()).collect();
            let edges: Vec<String> = n
                .draws
                .iter()
                .map(|e| {
                    format!(
                        "[{},{},{},{},{}]",
                        e.kind, e.w, e.to, e.score, e.synthetic as u8
                    )
                })
                .collect();
            out.push_str(&format!(
                "{{\"hand\":[{}],\"tenpai\":{},\"draws\":[{}]}}",
                hand_arr.join(","),
                n.tenpai,
                edges.join(",")
            ));
        }
        out.push_str("],\"n14\":[");
        for (i, n) in b.n14.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let discards: Vec<String> = n
                .discards
                .iter()
                .map(|(k, d)| format!("[{k},{d}]"))
                .collect();
            out.push_str(&format!(
                "{{\"win\":{},\"tenpai\":{},\"discards\":[{}]}}",
                n.win,
                n.tenpai,
                discards.join(",")
            ));
        }
        out.push_str("]}");
        out
    }
}

/// Parse an mpsz hand string ("045m123456p1167s") into a TileSet37.
pub fn hand_from_mpsz(s: &str) -> TileSet37 {
    TileSet37::from_iter(tiles_from_str(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    trait TileExt {
        fn from_str_checked(s: &str) -> Tile;
    }

    impl TileExt for Tile {
        fn from_str_checked(s: &str) -> Tile {
            s.parse().unwrap()
        }
    }

    #[test]
    fn analyze_gates_on_shanten() {
        let mut solver = Solver::new();
        // Tenpai: full DP.
        let hand = hand_from_mpsz("123456789m1112z");
        let cfg = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        match solver.analyze(&hand, &cfg) {
            Analysis::Full { stats, .. } => assert_eq!(stats.len(), 1),
            _ => panic!("tenpai hand must get the full DP"),
        }
        // Shanten 5-6 junk: simple mode, per-discard ukeire matching probes.
        let hand = hand_from_mpsz("147m258p369s1234z");
        let mut h14 = hand.0;
        h14[0] += 1; // second 1m -> 14 tiles
        let hand = TileSet37(h14);
        let cfg = SolverConfig::new(14, vec![Tile::from_str_checked("3p")]);
        match solver.analyze(&hand, &cfg) {
            Analysis::Simple { shanten, stats } => {
                assert!(shanten > SHANTEN_GATE);
                let held = (0..N_KINDS).filter(|&k| hand.0[k] > 0).count();
                assert_eq!(stats.len(), held);
                for s in &stats {
                    let k = s.tile.unwrap() as usize;
                    let mut h2 = hand.0;
                    h2[k] -= 1;
                    let expect = riichi_decomp::shanten(&fold34(&h2));
                    assert_eq!(s.shanten, expect, "discard {k}");
                    assert_eq!(s.shanten_down, expect > shanten, "discard {k}");
                    for &(t, _) in &s.necessary {
                        let mut h3 = fold34(&h2).0;
                        h3[t as usize] += 1;
                        assert!(
                            riichi_decomp::shanten(&TileSet34(h3)) < expect,
                            "discard {k}: tile {t} not advancing"
                        );
                    }
                }
            }
            _ => panic!("junk hand must get simple mode"),
        }
        // Reaction state (3N+1) beyond the gate: one current-hand stat.
        let hand = hand_from_mpsz("147m258p369s1234z");
        let cfg = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        match solver.analyze(&hand, &cfg) {
            Analysis::Simple { stats, .. } => {
                assert_eq!(stats.len(), 1);
                assert!(stats[0].tile.is_none());
                assert!(!stats[0].necessary.is_empty());
            }
            _ => panic!("junk reaction state must get simple mode"),
        }
    }

    #[test]
    fn analyze_preconditions() {
        let mut solver = Solver::new();
        // Complete hand.
        let hand = hand_from_mpsz("123456789m11122z");
        let cfg = SolverConfig::new(14, vec![Tile::from_str_checked("3p")]);
        assert!(matches!(solver.analyze(&hand, &cfg), Analysis::Unavailable));
        // No tsumos left.
        let hand = hand_from_mpsz("123456789m1112z");
        let mut cfg = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        cfg.t_max = 0;
        assert!(matches!(solver.analyze(&hand, &cfg), Analysis::Unavailable));
        // Wall smaller than the horizon.
        let mut cfg = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        cfg.t_max = 17;
        cfg.sum = 17;
        assert!(matches!(solver.analyze(&hand, &cfg), Analysis::Unavailable));
    }

    #[test]
    fn tenpai_hand_win_prob_matches_closed_form() {
        // 123456789m11p56s: 8 winning tiles, sum = 122.
        let hand = hand_from_mpsz("123456789m11p56s");
        let cfg = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        let mut solver = Solver::new();
        let (stats, _) = solver.solve(&hand, &cfg);
        assert_eq!(stats.len(), 1);
        let s = &stats[0];
        assert_eq!(s.shanten, 0);
        // v17 = 8 / (122 - 17)
        assert!((s.win_prob[17] - 8.0 / 105.0).abs() < 1e-12);
        assert_eq!(s.tenpai_prob[1], 1.0);
    }
}
