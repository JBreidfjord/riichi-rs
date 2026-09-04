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
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};

const PROFILE_BUILD_ID: &str = "v2-arena1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CacheMode {
    None,
    Score,
    Leaf,
    Arena,
}

impl CacheMode {
    fn from_env() -> Self {
        match std::env::var("RUSTCHI_SOLVER_CACHE_MODE").as_deref() {
            Ok("none") => Self::None,
            Ok("score") => Self::Score,
            Ok("all") | Ok("leaf") => Self::Leaf,
            Ok("arena") => Self::Arena,
            _ => Self::None,
        }
    }

    fn score(self) -> bool {
        matches!(self, Self::Score | Self::Leaf)
    }

    fn structure(self) -> bool {
        self == Self::Leaf

    }

    fn arena(self) -> bool {
        self == Self::Arena
    }

    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Score => "score",
            Self::Leaf => "leaf",
            Self::Arena => "arena",
        }
    }
}

fn cache_mode() -> CacheMode {
    static MODE: OnceLock<CacheMode> = OnceLock::new();
    *MODE.get_or_init(CacheMode::from_env)
}

/// Opt-in phase timers (env `RUSTCHI_ENCODE_PROFILE`), process-global.
pub mod profile {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::OnceLock;
    use std::time::Instant;

    static ENABLED: OnceLock<bool> = OnceLock::new();
    pub static FULL: AtomicU64 = AtomicU64::new(0);
    pub static SIMPLE: AtomicU64 = AtomicU64::new(0);
    pub static UNAVAILABLE: AtomicU64 = AtomicU64::new(0);
    pub static NS_GATE: AtomicU64 = AtomicU64::new(0);
    pub static NS_BUILD: AtomicU64 = AtomicU64::new(0);
    pub static NS_BUILD_INIT: AtomicU64 = AtomicU64::new(0);
    pub static NS_TOPOLOGY: AtomicU64 = AtomicU64::new(0);
    pub static NS_MATERIALIZE: AtomicU64 = AtomicU64::new(0);
    pub static NS_DP: AtomicU64 = AtomicU64::new(0);
    pub static NS_STATS: AtomicU64 = AtomicU64::new(0);
    pub static NS_SIMPLE: AtomicU64 = AtomicU64::new(0);
    pub static NS_SCORE_WIN: AtomicU64 = AtomicU64::new(0);
    pub static N_SCORE_WIN: AtomicU64 = AtomicU64::new(0);
    pub static SCORE_CACHE_HITS: AtomicU64 = AtomicU64::new(0);
    pub static SCORE_CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
    pub static STRUCTURE_CACHE_HITS: AtomicU64 = AtomicU64::new(0);
    pub static STRUCTURE_CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
    pub static NODES13: AtomicU64 = AtomicU64::new(0);
    pub static NODES14: AtomicU64 = AtomicU64::new(0);
    pub static TOPOLOGY_PEAK_NODES: AtomicU64 = AtomicU64::new(0);
    pub static TOPOLOGY_PEAK_BYTES: AtomicU64 = AtomicU64::new(0);
    pub static TOPOLOGY_CLEARS: AtomicU64 = AtomicU64::new(0);

    pub fn enabled() -> bool {
        *ENABLED.get_or_init(|| std::env::var_os("RUSTCHI_ENCODE_PROFILE").is_some())
    }

    pub fn start() -> Option<Instant> {
        enabled().then(Instant::now)
    }

    pub fn add(counter: &AtomicU64, start: Option<Instant>) {
        if let Some(t) = start {
            counter.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
        }
    }

    pub fn bump(counter: &AtomicU64, by: u64) {
        if enabled() {
            counter.fetch_add(by, Ordering::Relaxed);
        }
    }

    pub fn peak(counter: &AtomicU64, value: u64) {
        if enabled() {
            counter.fetch_max(value, Ordering::Relaxed);
        }
    }

    /// One-line human summary of everything since process start.
    pub fn report() -> Option<String> {
        if !enabled() {
            return None;
        }
        let g = |c: &AtomicU64| c.load(Ordering::Relaxed);
        let full = g(&FULL).max(1) as f64;
        let ms = |c: &AtomicU64| g(c) as f64 / 1e6;
        Some(format!(
            "solver profile: build_id={} cache_mode={}; analyses full={} simple={} unavailable={}; per FULL analysis: \
             build {:.0} us (init {:.0}, topology {:.0}, materialize {:.0}, score_win {:.0} us over {:.1} calls), dp {:.0} us, \
             stats {:.0} us; gate {:.0} us/analysis; simple path {:.0} us/analysis; \
             nodes per full: 13-tile {:.0}, 14-tile {:.0}; totals build {:.0} ms dp {:.0} ms \
             score_win {:.0} ms gate {:.0} ms simple {:.0} ms; score cache hits={} misses={}; \
             structure cache hits={} misses={}; topology peak={} nodes/~{} MiB clears={}",
            super::PROFILE_BUILD_ID,
            super::cache_mode().label(),
            g(&FULL),
            g(&SIMPLE),
            g(&UNAVAILABLE),
            g(&NS_BUILD) as f64 / full / 1e3,
            g(&NS_BUILD_INIT) as f64 / full / 1e3,
            g(&NS_TOPOLOGY) as f64 / full / 1e3,
            g(&NS_MATERIALIZE) as f64 / full / 1e3,
            g(&NS_SCORE_WIN) as f64 / full / 1e3,
            g(&N_SCORE_WIN) as f64 / full,
            g(&NS_DP) as f64 / full / 1e3,
            g(&NS_STATS) as f64 / full / 1e3,
            g(&NS_GATE) as f64 / (g(&FULL) + g(&SIMPLE) + g(&UNAVAILABLE)).max(1) as f64 / 1e3,
            g(&NS_SIMPLE) as f64 / g(&SIMPLE).max(1) as f64 / 1e3,
            g(&NODES13) as f64 / full,
            g(&NODES14) as f64 / full,
            ms(&NS_BUILD),
            ms(&NS_DP),
            ms(&NS_SCORE_WIN),
            ms(&NS_GATE),
            ms(&NS_SIMPLE),
            g(&SCORE_CACHE_HITS),
            g(&SCORE_CACHE_MISSES),
            g(&STRUCTURE_CACHE_HITS),
            g(&STRUCTURE_CACHE_MISSES),
            g(&TOPOLOGY_PEAK_NODES),
            g(&TOPOLOGY_PEAK_BYTES) / (1024 * 1024),
            g(&TOPOLOGY_CLEARS),
        ))
    }
}

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
///
/// This is the standard 136-tile set with one red five per numeral suit. It is the **default**
/// for [`SolverConfig`], not a law: see [`SolverConfig::copies`].
const fn default_kind_copies(k: usize) -> u8 {
    match k {
        4 | 13 | 22 => 3,
        RED_BASE..=36 => 1,
        _ => 4,
    }
}

/// The standard 136-tile copies table (one red five per numeral suit).
pub const DEFAULT_COPIES: [u8; N_KINDS] = {
    let mut t = [0u8; N_KINDS];
    let mut k = 0;
    while k < N_KINDS {
        t[k] = default_kind_copies(k);
        k += 1;
    }
    t
};

/// Number of tiles in the standard set.
pub const DEFAULT_WALL_SIZE: u32 = 136;

/// The standard dora chain, as a 34-kind indicator -> dora map.
///
/// Numerals wrap within their suit (`n % 9 + 1`), winds cycle E->S->W->N->E, dragons
/// haku->hatsu->chun->haku. It is the **default** for [`SolverConfig`], not a law: sanma's manzu
/// chain is 1m <-> 9m, because 2m--8m do not exist.
pub const DEFAULT_DORA_MAP: [u8; 34] = [
    1, 2, 3, 4, 5, 6, 7, 8, 0, // m
    10, 11, 12, 13, 14, 15, 16, 17, 9, // p
    19, 20, 21, 22, 23, 24, 25, 26, 18, // s
    28, 29, 30, 27, // winds
    32, 33, 31, // dragons
];

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

const SCORE_CACHE_CAPACITY: usize = 16384;
const SCORE_CONTEXT_CAPACITY: usize = 256;

#[derive(Eq, PartialEq)]
struct ScoringContext {
    dora_indicators: Vec<u8>,
    round_kyoku: u8,
    round_honba: u8,
    seat: u8,
    bonus_han: u8,
    enable_uradora: bool,
    melds: Vec<u8>,
    dora_map: [u8; 34],
    copies: [u8; N_KINDS],
}

impl ScoringContext {
    fn new(cfg: &SolverConfig) -> Self {
        let mut melds = Vec::new();
        for meld in &cfg.melds {
            let tag = match meld {
                Meld::Chii(_) => 0,
                Meld::Pon(_) => 1,
                Meld::Kakan(_) => 2,
                Meld::Daiminkan(_) => 3,
                Meld::Ankan(_) => 4,
                Meld::Kita(_) => 5,
            };
            let tiles = meld.to_tiles();
            melds.extend([tag, tiles.len() as u8]);
            melds.extend(tiles.into_iter().map(|tile| tile.encoding()));
            melds.push(meld.called().map_or(u8::MAX, |tile| tile.encoding()));
            melds.push(meld.dir().map_or(u8::MAX, |dir| dir.to_usize() as u8));
        }
        Self {
            dora_indicators: cfg.dora_indicators.iter().map(|tile| tile.encoding()).collect(),
            round_kyoku: cfg.round_id.kyoku,
            round_honba: cfg.round_id.honba,
            seat: cfg.seat.to_usize() as u8,
            bonus_han: cfg.bonus_han,
            enable_uradora: cfg.enable_uradora,
            melds,
            dora_map: cfg.dora_map,
            copies: cfg.copies,
        }
    }
}

type ScoreKey = (u64, u128, u8, bool);

struct ScoreCache {
    contexts: VecDeque<(ScoringContext, u64)>,
    next_context_id: u64,
    clock: u64,
    values: HashMap<ScoreKey, (f64, u64)>,
    recency: VecDeque<(ScoreKey, u64)>,
}

impl Default for ScoreCache {
    fn default() -> Self {
        Self {
            contexts: VecDeque::new(),
            next_context_id: 0,
            clock: 0,
            values: HashMap::default(),
            recency: VecDeque::new(),
        }
    }
}

impl ScoreCache {
    fn intern_context(&mut self, context: ScoringContext) -> u64 {
        if let Some((_, id)) = self.contexts.iter().find(|(known, _)| known == &context) {
            return *id;
        }
        let id = self.next_context_id;
        self.next_context_id = self.next_context_id.wrapping_add(1);
        if self.contexts.len() == SCORE_CONTEXT_CAPACITY {
            self.contexts.pop_front();
        }
        self.contexts.push_back((context, id));
        id
    }

    fn get(&mut self, key: ScoreKey) -> Option<f64> {
        let (value, generation) = self.values.get_mut(&key)?;
        self.clock = self.clock.wrapping_add(1);
        *generation = self.clock;
        self.recency.push_back((key, self.clock));
        Some(*value)
    }

    fn insert(&mut self, key: ScoreKey, value: f64) {
        self.clock = self.clock.wrapping_add(1);
        self.values.insert(key, (value, self.clock));
        self.recency.push_back((key, self.clock));
        while self.values.len() > SCORE_CACHE_CAPACITY {
            let Some((old_key, old_generation)) = self.recency.pop_front() else {
                break;
            };
            if self.values.get(&old_key).map(|(_, generation)| *generation) == Some(old_generation)
            {
                self.values.remove(&old_key);
            }
        }
    }
}

thread_local! {
    static SCORE_CACHE: RefCell<ScoreCache> = RefCell::new(ScoreCache::default());
    static STRUCTURE_CACHE: RefCell<StructureCache> = RefCell::new(StructureCache::default());
}

const STRUCTURE_CACHE_CAPACITY: usize = 65536;

#[derive(Clone)]
struct Structure13 {
    shanten: i8,
    tenpai: bool,
    draw_kinds: Vec<u8>,
    wait_set: Option<WaitSet>,
}

#[derive(Clone)]
struct Structure14 {
    win: bool,
    tenpai: bool,
    discards: Vec<(u8, [u8; N_KINDS])>,
}

#[derive(Default)]
struct StructureCache {
    n13: HashMap<u128, Arc<Structure13>>,
    n14: HashMap<u128, Arc<Structure14>>,
}

impl StructureCache {
    fn get13(&mut self, hand_key: u128) -> Option<Arc<Structure13>> {
        self.n13.get(&hand_key).map(Arc::clone)
    }

    fn insert13(&mut self, hand_key: u128, value: Arc<Structure13>) {
        if self.n13.len() >= STRUCTURE_CACHE_CAPACITY {
            self.n13.clear();
        }
        self.n13.insert(hand_key, value);
    }

    fn get14(&mut self, hand_key: u128) -> Option<Arc<Structure14>> {
        self.n14.get(&hand_key).map(Arc::clone)
    }

    fn insert14(&mut self, hand_key: u128, value: Arc<Structure14>) {
        if self.n14.len() >= STRUCTURE_CACHE_CAPACITY {
            self.n14.clear();
        }
        self.n14.insert(hand_key, value);
    }
}

const TOPOLOGY_ARENA_CAPACITY: usize = 131072;
type TopologyKey = (u64, u128);

struct Topology13 {
    hand: [u8; N_KINDS],
    shanten: i8,
    tenpai: bool,
    draws: Vec<(u8, usize)>,
    wait_set: Option<Arc<WaitSet>>,
    visit_generation: u64,
    local_id: usize,
}

struct Topology14 {
    win: bool,
    tenpai: bool,
    discards: Vec<(u8, usize)>,
    visit_generation: u64,
    local_id: usize,
}

#[derive(Default)]
struct TopologyArena {
    n13: Vec<Topology13>,
    n14: Vec<Topology14>,
    memo13: HashMap<TopologyKey, usize>,
    memo14: HashMap<TopologyKey, usize>,
    generation: u64,
    estimated_bytes: usize,
}

impl TopologyArena {
    fn clear_if_full(&mut self) -> bool {
        if self.n13.len() + self.n14.len() < TOPOLOGY_ARENA_CAPACITY {
            return false;
        }
        self.n13.clear();
        self.n14.clear();
        self.memo13.clear();
        self.memo14.clear();
        self.estimated_bytes = 0;
        true
    }

    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.generation
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SolverConfig {
    pub t_max: usize,
    /// Total wall tiles (unseen from the root hand's perspective).
    pub sum: u32,

    /// Copies of each 37-encoded kind present in the **full tile set**.
    ///
    /// This is deliberately *data*, not a rules axis: it keeps this crate a pure combinatorial
    /// engine, testable without a `Ruleset`, and it is the one input that makes a 3-player tile
    /// set work. A sanma table has **0** for kinds 1..=7 (2m--8m), 0 for kind 4 (5m) and 0 for
    /// kind 34 (red 5m); everything else stays 4 (or 3/1 for the remaining fives and reds).
    ///
    /// Without it, `wall_count` assumes 4 copies of everything and the whole solver computes
    /// ukeire, win probability and EV over 28 phantom manzu tiles, offering them as improving
    /// draws.
    ///
    /// Callers should derive this in exactly one place --- `riichi_elements::Variant` has
    /// `num_copies_34` for the tile-kind half; the red-five split is the caller's, since the
    /// number of reds is implied by the wall array rather than by the variant.
    #[serde(with = "BigArray")]
    pub copies: [u8; N_KINDS],

    /// Indicator -> dora map over the 34 normal kinds.
    ///
    /// Also data. Sanma's manzu chain is 1m <-> 9m rather than `n % 9 + 1`, because 2m--8m are
    /// not in the set --- a solver told the wrong chain misvalues every hand holding a terminal
    /// manzu whenever a manzu indicator is up.
    #[serde(with = "BigArray")]
    pub dora_map: [u8; 34],

    /// Flat han added to every winning hand, before dora.
    ///
    /// This exists for sanma's own Kita (北抜き): each extracted North is `+1` han, but the North
    /// is **no longer in the hand**, so no indicator arithmetic reproduces it (a West indicator
    /// would make every North still *in hand* dora instead). Left at 0, a hand with kita reads
    /// low in proportion to how many were extracted --- biased exactly when the hand is worth
    /// most.
    pub bonus_han: u8,

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
    /// Reference-parity defaults for a given root hand size (closed tiles), over the standard
    /// 136-tile set.
    pub fn new(root_tiles: u32, dora_indicators: Vec<Tile>) -> Self {
        Self::new_in(
            DEFAULT_WALL_SIZE,
            DEFAULT_COPIES,
            DEFAULT_DORA_MAP,
            root_tiles,
            dora_indicators,
        )
    }

    /// As [`Self::new`], but over an explicitly described tile set.
    ///
    /// `wall_size` is how many tiles the full set has (136, or 108 for sanma); `copies` and
    /// `dora_map` describe its shape. See [`Self::copies`] and [`Self::dora_map`].
    pub fn new_in(
        wall_size: u32,
        copies: [u8; N_KINDS],
        dora_map: [u8; 34],
        root_tiles: u32,
        dora_indicators: Vec<Tile>,
    ) -> Self {
        debug_assert_eq!(
            copies.iter().map(|&c| c as u32).sum::<u32>(),
            wall_size,
            "copies table does not add up to the stated wall size"
        );
        let sum = wall_size - root_tiles - dora_indicators.len() as u32;
        SolverConfig {
            t_max: T_MAX,
            sum,
            copies,
            dora_map,
            bonus_han: 0,
            dora_indicators,
            round_id: RoundId { kyoku: 0, honba: 0 },
            seat: Player::new(1),
            enable_uradora: true,
            melds: Vec::new(),
        }
    }

    /// Attach a flat han bonus; see [`Self::bonus_han`].
    pub fn with_bonus_han(mut self, bonus_han: u8) -> Self {
        self.bonus_han = bonus_han;
        self
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
#[derive(Debug, Deserialize, PartialEq, Serialize)]
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
    topology: TopologyArena,
    cache_mode: CacheMode,
    scratch_n13: Vec<Node13>,
    scratch_n14: Vec<Node14>,
    scratch_memo13: HashMap<u128, usize>,
    scratch_memo14: HashMap<u128, usize>,
    scratch_v13: Vec<[f64; 3]>,
    scratch_v14: Vec<[f64; 3]>,
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
    score_context_id: u64,
    cache_mode: CacheMode,
    score_cache_hits: u64,
    score_cache_misses: u64,
    structure_cache_hits: u64,
    structure_cache_misses: u64,
}

impl<'a> Build<'a> {
    fn new(
        cfg: &'a SolverConfig,
        root_hand: &[u8; N_KINDS],
        cache_mode: CacheMode,
    ) -> Build<'a> {
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
            let d = cfg.dora_map[t.normal_encoding() as usize] as usize;
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
        let score_context_id = if cache_mode.score() {
            SCORE_CACHE.with(|cache| cache.borrow_mut().intern_context(ScoringContext::new(cfg)))
        } else {
            0
        };
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
            score_context_id,
            cache_mode,
            score_cache_hits: 0,
            score_cache_misses: 0,
            structure_cache_hits: 0,
            structure_cache_misses: 0,
        }
    }

}

impl Solver {
    pub fn new() -> Self {
        Solver {
            decomposer: Decomposer::new(),
            ruleset: Ruleset::default(),
            lut: ShantenLut::get(),
            topology: TopologyArena::default(),
            cache_mode: cache_mode(),
            scratch_n13: Vec::new(),
            scratch_n14: Vec::new(),
            scratch_memo13: HashMap::default(),
            scratch_memo14: HashMap::default(),
            scratch_v13: Vec::new(),
            scratch_v14: Vec::new(),
        }
    }

    /// Solve a root hand (13 or 14 tiles, closed). Returns per-candidate stats
    /// and the number of graph vertices searched.
    pub fn solve(&mut self, hand: &TileSet37, cfg: &SolverConfig) -> (Vec<Stat>, usize) {
        let root_hand = hand.0;
        let n_tiles: u32 = fold34(&root_hand).0.iter().map(|&c| c as u32).sum();
        let t_build = profile::start();
        let t_build_init = profile::start();
        let mut b = Build::new(cfg, &root_hand, self.cache_mode);
        b.n13 = std::mem::take(&mut self.scratch_n13);
        b.n14 = std::mem::take(&mut self.scratch_n14);
        b.memo13 = std::mem::take(&mut self.scratch_memo13);
        b.memo14 = std::mem::take(&mut self.scratch_memo14);
        b.n13.clear();
        b.n14.clear();
        b.memo13.clear();
        b.memo14.clear();
        profile::add(&profile::NS_BUILD_INIT, t_build_init);

        let t_topology = profile::start();
        let arena_root = if b.cache_mode.arena() {
            if self.topology.clear_if_full() {
                profile::bump(&profile::TOPOLOGY_CLEARS, 1);
            }
            let positive_mask = cfg
                .copies
                .iter()
                .enumerate()
                .fold(0u64, |mask, (kind, &copies)| mask | ((copies > 0) as u64) << kind);
            Some(if n_tiles % 3 == 1 {
                (true, self.ensure_topology13(&mut b, root_hand, positive_mask))
            } else {
                (false, self.ensure_topology14(&mut b, root_hand, positive_mask))
            })
        } else {
            None
        };
        profile::add(&profile::NS_TOPOLOGY, t_topology);
        let generation = arena_root.map(|_| self.topology.next_generation());

        let t_materialize = profile::start();
        let root_stats: Vec<(Option<u8>, usize)> = if n_tiles % 3 == 1 {
            let id = if let Some((true, topology_id)) = arena_root {
                self.build13_from_topology(&mut b, topology_id, generation.unwrap())
            } else {
                self.build13(&mut b, root_hand)
            };
            vec![(None, id)]
        } else {
            // 14 tiles: the root is itself a graph node; its min-shanten
            // discards are the candidates.
            let root_id = if let Some((false, topology_id)) = arena_root {
                self.build14_from_topology(&mut b, topology_id, generation.unwrap())
            } else {
                self.build14(&mut b, root_hand)
            };
            b.n14[root_id]
                .discards
                .clone()
                .into_iter()
                .map(|(k, id)| (Some(k), id))
                .collect()
        };
        profile::add(&profile::NS_MATERIALIZE, t_materialize);

        profile::add(&profile::NS_BUILD, t_build);
        profile::bump(&profile::SCORE_CACHE_HITS, b.score_cache_hits);
        profile::bump(&profile::SCORE_CACHE_MISSES, b.score_cache_misses);
        profile::bump(&profile::STRUCTURE_CACHE_HITS, b.structure_cache_hits);
        profile::bump(&profile::STRUCTURE_CACHE_MISSES, b.structure_cache_misses);
        profile::bump(&profile::NODES13, b.n13.len() as u64);
        profile::bump(&profile::NODES14, b.n14.len() as u64);
        profile::peak(
            &profile::TOPOLOGY_PEAK_NODES,
            (self.topology.n13.len() + self.topology.n14.len()) as u64,
        );
        profile::peak(&profile::TOPOLOGY_PEAK_BYTES, self.topology.estimated_bytes as u64);
        let t_dp = profile::start();
        let (v13, v14) = self.run_dp(&b);
        profile::add(&profile::NS_DP, t_dp);
        let t_stats = profile::start();

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
                Stat { tile, shanten: node.shanten, tenpai_prob, win_prob, exp_score, necessary }
            })
            .collect();
        profile::add(&profile::NS_STATS, t_stats);

        let searched = b.n13.len() + b.n14.len();
        self.scratch_n13 = b.n13;
        self.scratch_n14 = b.n14;
        self.scratch_memo13 = b.memo13;
        self.scratch_memo14 = b.memo14;
        self.scratch_v13 = v13;
        self.scratch_v14 = v14;
        (stats, searched)
    }

    /// Wall copies of kind `k` unseen from `hand`'s perspective.
    fn wall_count(b: &Build, hand: &[u8; N_KINDS], k: usize) -> u8 {
        let ind = if k < 34 { b.ind34[k] } else { 0 };
        b.cfg.copies[k].saturating_sub(hand[k]).saturating_sub(ind).saturating_sub(b.meld37[k])
    }

    fn ensure_topology13(
        &mut self,
        b: &mut Build,
        hand: [u8; N_KINDS],
        positive_mask: u64,
    ) -> usize {
        let topology_key = (positive_mask, key(&hand));
        if let Some(&id) = self.topology.memo13.get(&topology_key) {
            b.structure_cache_hits += 1;
            return id;
        }
        b.structure_cache_misses += 1;
        let id = self.topology.n13.len();
        self.topology.memo13.insert(topology_key, id);
        self.topology.n13.push(Topology13 {
            hand,
            shanten: 0,
            tenpai: false,
            draws: Vec::new(),
            wait_set: None,
            visit_generation: 0,
            local_id: 0,
        });

        let folded = fold34(&hand);
        let (shanten, advancing) = self.lut.analyze_13(&folded);
        let tenpai = shanten == 0;
        let wait_set =
            tenpai.then(|| Arc::new(WaitSet::from_tile_set(&mut self.decomposer, &folded)));
        let mut draws = Vec::new();
        for kind in 0..N_KINDS {
            if positive_mask & (1 << kind) == 0
                || advancing & (1 << fold_kind(kind)) == 0
            {
                continue;
            }
            let mut child = hand;
            child[kind] += 1;
            let child_id = self.ensure_topology14(b, child, positive_mask);
            draws.push((kind as u8, child_id));
        }
        self.topology.estimated_bytes += std::mem::size_of::<Topology13>()
            + draws.capacity() * std::mem::size_of::<(u8, usize)>()
            + wait_set.as_ref().map_or(0, |_| std::mem::size_of::<WaitSet>());
        let node = &mut self.topology.n13[id];
        node.shanten = shanten;
        node.tenpai = tenpai;
        node.draws = draws;
        node.wait_set = wait_set;
        id
    }

    fn ensure_topology14(
        &mut self,
        b: &mut Build,
        hand: [u8; N_KINDS],
        positive_mask: u64,
    ) -> usize {
        let topology_key = (positive_mask, key(&hand));
        if let Some(&id) = self.topology.memo14.get(&topology_key) {
            b.structure_cache_hits += 1;
            return id;
        }
        b.structure_cache_misses += 1;
        let id = self.topology.n14.len();
        self.topology.memo14.insert(topology_key, id);
        self.topology.n14.push(Topology14 {
            win: false,
            tenpai: false,
            discards: Vec::new(),
            visit_generation: 0,
            local_id: 0,
        });

        let (shanten, keep) = self.lut.analyze_14(&fold34(&hand));
        let mut discards = Vec::new();
        if shanten != -1 {
            for kind in 0..N_KINDS {
                if hand[kind] == 0 || keep & (1 << fold_kind(kind)) == 0 {
                    continue;
                }
                let mut child = hand;
                child[kind] -= 1;
                let child_id = self.ensure_topology13(b, child, positive_mask);
                discards.push((kind as u8, child_id));
            }
        }
        self.topology.estimated_bytes += std::mem::size_of::<Topology14>()
            + discards.capacity() * std::mem::size_of::<(u8, usize)>();
        let node = &mut self.topology.n14[id];
        node.win = shanten == -1;
        node.tenpai = shanten <= 0;
        node.discards = discards;
        id
    }

    fn build13_from_topology(
        &mut self,
        b: &mut Build,
        topology_id: usize,
        generation: u64,
    ) -> usize {
        let topology = &mut self.topology.n13[topology_id];
        if topology.visit_generation == generation {
            return topology.local_id;
        }
        let id = b.n13.len();
        topology.visit_generation = generation;
        topology.local_id = id;
        let hand = topology.hand;
        let shanten = topology.shanten;
        let tenpai = topology.tenpai;
        let draw_topology = topology.draws.clone();
        let wait_set = topology.wait_set.clone();
        b.n13.push(Node13 { hand, shanten, tenpai, draws: Vec::new() });

        let hk = key(&hand);
        let mut draws = Vec::new();
        for (kind, child_topology) in draw_topology {
            let k = kind as usize;
            let w = Self::wall_count(b, &hand, k);
            if w == 0 {
                continue;
            }
            let score = if tenpai {
                let riichi = b.menzen && b.root13_key != Some(hk);
                self.score_win(b, &hand, wait_set.as_ref().unwrap(), k, riichi)
            } else {
                0.0
            };
            let to = self.build14_from_topology(b, child_topology, generation);
            b.n14[to].undo.push(id);
            draws.push(DrawEdge { kind, w, to, score, synthetic: false });
        }
        b.n13[id].draws = draws;
        id
    }

    fn build14_from_topology(
        &mut self,
        b: &mut Build,
        topology_id: usize,
        generation: u64,
    ) -> usize {
        let topology = &mut self.topology.n14[topology_id];
        if topology.visit_generation == generation {
            return topology.local_id;
        }
        let id = b.n14.len();
        topology.visit_generation = generation;
        topology.local_id = id;
        let win = topology.win;
        let tenpai = topology.tenpai;
        let discard_topology = topology.discards.clone();
        b.n14.push(Node14 { win, tenpai, discards: Vec::new(), undo: Vec::new() });

        let mut discards = Vec::new();
        for (kind, child_topology) in discard_topology {
            let child = self.build13_from_topology(b, child_topology, generation);
            let w = Self::wall_count(b, &b.n13[child].hand, kind as usize);
            if w > 0 {
                b.n13[child].draws.push(DrawEdge {
                    kind,
                    w,
                    to: id,
                    score: 0.0,
                    synthetic: true,
                });
            }
            discards.push((kind, child));
        }
        b.n14[id].discards = discards;
        id
    }

    fn structure13(
        &mut self,
        hand: &[u8; N_KINDS],
        use_cache: bool,
    ) -> (Arc<Structure13>, bool) {
        let hand_key = key(hand);
        if use_cache {
            if let Some(value) =
                STRUCTURE_CACHE.with(|cache| cache.borrow_mut().get13(hand_key))
            {
                return (value, true);
            }
        }
        let folded = fold34(hand);
        let (shanten, advancing) = self.lut.analyze_13(&folded);
        let tenpai = shanten == 0;
        let value = Arc::new(Structure13 {
            shanten,
            tenpai,
            draw_kinds: (0..N_KINDS)
                .filter(|&kind| advancing & (1 << fold_kind(kind)) != 0)
                .map(|kind| kind as u8)
                .collect(),
            wait_set: tenpai.then(|| WaitSet::from_tile_set(&mut self.decomposer, &folded)),
        });
        if use_cache {
            STRUCTURE_CACHE
                .with(|cache| cache.borrow_mut().insert13(hand_key, Arc::clone(&value)));
        }
        (value, false)
    }

    fn structure14(
        &mut self,
        hand: &[u8; N_KINDS],
        use_cache: bool,
    ) -> (Arc<Structure14>, bool) {
        let hand_key = key(hand);
        if use_cache {
            if let Some(value) =
                STRUCTURE_CACHE.with(|cache| cache.borrow_mut().get14(hand_key))
            {
                return (value, true);
            }
        }
        let (shanten, keep) = self.lut.analyze_14(&fold34(hand));
        let discards = if shanten == -1 {
            Vec::new()
        } else {
            (0..N_KINDS)
                .filter(|&kind| hand[kind] > 0 && keep & (1 << fold_kind(kind)) != 0)
                .map(|kind| {
                    let mut child = *hand;
                    child[kind] -= 1;
                    (kind as u8, child)
                })
                .collect()
        };
        let value = Arc::new(Structure14 {
            win: shanten == -1,
            tenpai: shanten <= 0,
            discards,
        });
        if use_cache {
            STRUCTURE_CACHE
                .with(|cache| cache.borrow_mut().insert14(hand_key, Arc::clone(&value)));
        }
        (value, false)
    }

    fn build13(&mut self, b: &mut Build, hand: [u8; N_KINDS]) -> usize {
        let hk = key(&hand);
        if let Some(&id) = b.memo13.get(&hk) {
            return id;
        }
        let id = b.n13.len();
        b.n13.push(Node13 { hand, shanten: 0, tenpai: false, draws: Vec::new() });
        b.memo13.insert(hk, id);

        let (structure, cache_hit) = self.structure13(&hand, b.cache_mode.structure());
        if cache_hit {
            b.structure_cache_hits += 1;
        } else {
            b.structure_cache_misses += 1;
        }

        let mut draws = Vec::new();
        for &kind in &structure.draw_kinds {
            let k = kind as usize;
            let w = Self::wall_count(b, &hand, k);
            if w == 0 {
                continue;
            }
            let mut h2 = hand;
            h2[k] += 1;
            let score = if structure.tenpai {
                // Riichi requires menzen; the 13-tile root additionally has
                // not declared yet (declaration happens on a discard).
                let riichi = b.menzen && b.root13_key != Some(hk);
                self.score_win(b, &hand, structure.wait_set.as_ref().unwrap(), k, riichi)
            } else {
                0.0
            };
            let to = self.build14(b, h2);
            b.n14[to].undo.push(id);
            draws.push(DrawEdge { kind: k as u8, w, to, score, synthetic: false });
        }
        b.n13[id].shanten = structure.shanten;
        b.n13[id].tenpai = structure.tenpai;
        b.n13[id].draws = draws;
        id
    }

    fn build14(&mut self, b: &mut Build, hand: [u8; N_KINDS]) -> usize {
        let hk = key(&hand);
        if let Some(&id) = b.memo14.get(&hk) {
            return id;
        }
        let id = b.n14.len();
        b.n14.push(Node14 { win: false, tenpai: false, discards: Vec::new(), undo: Vec::new() });
        b.memo14.insert(hk, id);

        let (structure, cache_hit) = self.structure14(&hand, b.cache_mode.structure());
        if cache_hit {
            b.structure_cache_hits += 1;
        } else {
            b.structure_cache_misses += 1;
        }
        // Win nodes are terminal: the hand is closed and tenpai, so riichi is
        // locked — continuing can never beat taking the win (same waits, same
        // score, fewer draws left). Expanding their discards would also let
        // the graph wander between tenpai hands unboundedly.
        let discards: Vec<(u8, usize)> = {
            // Each discard edge also gets a reverse draw edge (redrawing the
            // discarded tile back into this 14-tile hand), letting the DP
            // revisit the discard choice later. This adds edges between
            // existing vertices only, and never duplicates an advancing
            // draw: a min-shanten discard means parent and child shanten are
            // equal, so the redraw is shanten-neutral.
            structure.discards.iter()
                .map(|&(k, h)| {
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
        n.win = structure.win;
        n.tenpai = structure.tenpai;
        n.discards = discards;
        id
    }

    /// Expected winner gain for drawing `draw_kind` into tenpai `hand13`,
    /// including exact uradora EV when riichi.
    fn score_win(
        &mut self,
        b: &mut Build,
        hand13: &[u8; N_KINDS],
        wait_set: &WaitSet,
        draw_kind: usize,
        riichi: bool,
    ) -> f64 {
        let t = profile::start();
        let cache_key = (b.score_context_id, key(hand13), draw_kind as u8, riichi);
        if b.cache_mode.score() {
            if let Some(value) = SCORE_CACHE.with(|cache| cache.borrow_mut().get(cache_key)) {
                b.score_cache_hits += 1;
                profile::add(&profile::NS_SCORE_WIN, t);
                profile::bump(&profile::N_SCORE_WIN, 1);
                return value;
            }
        }
        b.score_cache_misses += 1;
        let r = self.score_win_inner(b, hand13, wait_set, draw_kind, riichi);
        if b.cache_mode.score() {
            SCORE_CACHE.with(|cache| cache.borrow_mut().insert(cache_key, r));
        }
        profile::add(&profile::NS_SCORE_WIN, t);
        profile::bump(&profile::N_SCORE_WIN, 1);
        r
    }

    fn score_win_inner(
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
            variant: self.ruleset.variant,
            round_id: b.cfg.round_id,
            winner: b.cfg.seat,
            closed_hand: &closed,
            riichi: riichi.then_some(riichi::model::Riichi { is_double: false, is_ippatsu: false }),
            melds: std::borrow::Cow::Borrowed(&b.cfg.melds),
            wait_set,
            contributor: b.cfg.seat,
            incoming_draws_from_tail: false,
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
            .map(|i| all34.0[b.cfg.dora_map[i.normal_encoding() as usize] as usize])
            .sum::<u8>()
            + b.meld_dora;
        let aka: u8 = hand14[34] + hand14[35] + hand14[36] + b.meld_aka;

        let gain = |ura: u8| -> f64 {
            let dh = DoraHits {
                dora,
                ura_dora: ura,
                aka_dora: aka,
                // Own Kita: +1 han each, flat. See `SolverConfig::bonus_han`.
                nuki_dora: b.cfg.bonus_han,
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
            distribute_points(&self.ruleset, b.cfg.round_id, false, b.cfg.seat, b.cfg.seat, basic)
                [b.cfg.seat.to_usize()] as f64
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
            let di = b.cfg.dora_map[i] as usize;
            let gi = (all34.0[di] + b.meld34[di]) as usize;
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
    fn run_dp(&mut self, b: &Build) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
        let t_max = b.cfg.t_max;
        let stride = t_max + 1;
        let mut v13 = std::mem::take(&mut self.scratch_v13);
        let mut v14 = std::mem::take(&mut self.scratch_v14);
        v13.resize(b.n13.len() * stride, [0.0; 3]);
        v14.resize(b.n14.len() * stride, [0.0; 3]);
        v13.fill([0.0; 3]);
        v14.fill([0.0; 3]);

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

    fn node14_value(n: &Node14, v13: &[[f64; 3]], stride: usize, t: usize) -> [f64; 3] {
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
#[derive(Debug, Deserialize, PartialEq, Serialize)]
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

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub enum Analysis {
    /// Root shanten <= [`SHANTEN_GATE`]: full tenpai/win/EV tables.
    Full { stats: Vec<Stat>, searched: usize },
    /// Beyond the gate: ukeire only, no DP. `shanten` is the root's.
    Simple { shanten: i8, stats: Vec<SimpleStat> },
    /// Preconditions failed: no tsumos left, wall too small for the horizon,
    /// or the hand is already complete. Encoder-side fallback applies.
    Unavailable,
}

/// One exact solver-boundary capture record, suitable for JSONL replay.
#[derive(Debug, Deserialize, Serialize)]
pub struct CaptureRecord {
    #[serde(with = "BigArray")]
    pub hand: [u8; N_KINDS],
    pub cfg: SolverConfig,
    pub analysis: Analysis,
}

impl Solver {
    /// Production entry point: preconditions, then the shanten gate decides
    /// full DP vs ukeire-only simple mode. Works for action states (3N+2
    /// closed tiles: per-candidate stats) and reaction states (3N+1: a
    /// single current-hand stat).
    pub fn analyze(&mut self, hand: &TileSet37, cfg: &SolverConfig) -> Analysis {
        let t_gate = profile::start();
        let folded = fold34(&hand.0);
        let n_tiles: u32 = folded.0.iter().map(|&c| c as u32).sum();
        if cfg.t_max == 0 || (cfg.sum as usize) <= cfg.t_max {
            profile::add(&profile::NS_GATE, t_gate);
            profile::bump(&profile::UNAVAILABLE, 1);
            return Analysis::Unavailable;
        }
        let shanten = if n_tiles % 3 == 2 {
            self.lut.analyze_14(&folded).0
        } else {
            self.lut.analyze_13(&folded).0
        };
        profile::add(&profile::NS_GATE, t_gate);
        if shanten == -1 {
            profile::bump(&profile::UNAVAILABLE, 1);
            return Analysis::Unavailable;
        }
        if shanten <= SHANTEN_GATE {
            profile::bump(&profile::FULL, 1);
            let (stats, searched) = self.solve(hand, cfg);
            return Analysis::Full { stats, searched };
        }

        profile::bump(&profile::SIMPLE, 1);
        let t_simple = profile::start();
        let b = Build::new(cfg, &hand.0, self.cache_mode);
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
        profile::add(&profile::NS_SIMPLE, t_simple);
        Analysis::Simple { shanten, stats }
    }

    /// Encoder fallback when [`Solver::analyze`] returns
    /// [`Analysis::Unavailable`]: the current hand scored as a tsumo win with
    /// no riichi and no uradora — the minimal interpretation. `hand` must be
    /// a 3N+2 root that includes `winning_tile`. Returns 0 unless the hand is
    /// a yaku-bearing agari on `winning_tile`.
    pub fn min_tsumo_points(
        &mut self,
        hand: &TileSet37,
        winning_tile: Tile,
        cfg: &SolverConfig,
    ) -> f64 {
        let mut hand13 = hand.0;
        let wk = winning_tile.encoding() as usize;
        if hand13[wk] == 0 {
            return 0.0;
        }
        hand13[wk] -= 1;
        let folded13 = fold34(&hand13);
        let n_tiles: u32 = folded13.0.iter().map(|&c| c as u32).sum();
        if n_tiles % 3 != 1 {
            return 0.0;
        }
        let wait_set = WaitSet::from_tile_set(&mut self.decomposer, &folded13);
        if !wait_set.waiting_tiles.has(winning_tile.to_normal()) {
            return 0.0;
        }
        let closed = to_tileset37(&hand13);
        let input = AgariInput {
            variant: self.ruleset.variant,
            round_id: cfg.round_id,
            winner: cfg.seat,
            closed_hand: &closed,
            riichi: None,
            melds: std::borrow::Cow::Borrowed(&cfg.melds),
            wait_set: &wait_set,
            contributor: cfg.seat,
            incoming_draws_from_tail: false,
            action_is_kan: false,
            winning_tile,
            is_first_chance: false,
            is_last_draw: false,
        };
        let candidates = agari_candidates(&self.ruleset, &input);
        if candidates.is_empty() {
            return 0.0; // yakuless
        }

        let b = Build::new(cfg, &hand13, self.cache_mode);
        let all34 = fold34(&hand.0);
        let dora: u8 = cfg
            .dora_indicators
            .iter()
            .map(|i| all34.0[cfg.dora_map[i.normal_encoding() as usize] as usize])
            .sum::<u8>()
            + b.meld_dora;
        let aka: u8 = hand.0[34] + hand.0[35] + hand.0[36] + b.meld_aka;
        let dh = DoraHits { dora, ura_dora: 0, aka_dora: aka, nuki_dora: cfg.bonus_han };
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
        distribute_points(&self.ruleset, cfg.round_id, false, cfg.seat, cfg.seat, basic)
            [cfg.seat.to_usize()] as f64
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
        let mut b = Build::new(cfg, &root_hand, self.cache_mode);
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
                .map(|e| format!("[{},{},{},{},{}]", e.kind, e.w, e.to, e.score, e.synthetic as u8))
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
            let discards: Vec<String> =
                n.discards.iter().map(|(k, d)| format!("[{k},{d}]")).collect();
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

    fn clear_solver_caches() {
        SCORE_CACHE.with(|cache| *cache.borrow_mut() = ScoreCache::default());
        STRUCTURE_CACHE.with(|cache| *cache.borrow_mut() = StructureCache::default());
    }

    trait TileExt {
        fn from_str_checked(s: &str) -> Tile;
    }

    impl TileExt for Tile {
        fn from_str_checked(s: &str) -> Tile {
            s.parse().unwrap()
        }
    }

    /// Build the sanma tile set as an explicit copies table: no 2m--8m, no 5m, no red 5m.
    fn sanma_copies() -> [u8; N_KINDS] {
        let mut c = DEFAULT_COPIES;
        for k in 1..=7 {
            c[k] = 0;
        } // 2m..8m
        c[RED_BASE] = 0; // red 5m (kind 4 is 5m, already zeroed above)
        c
    }

    fn sanma_dora_map() -> [u8; 34] {
        let mut m = DEFAULT_DORA_MAP;
        m[0] = 8; // 1m -> 9m
        m[8] = 0; // 9m -> 1m
        m
    }

    /// The whole point of ADR 0010's copies table: without it the solver treats 2m--8m as
    /// unseen wall tiles and computes ukeire, win probability and EV over 28 tiles that are not
    /// in the game.
    ///
    /// Note the test cannot be "yonma offers 2m as an improving draw and sanma does not": any
    /// hand shape that *wants* a 2m is by construction not a legal sanma hand. What is testable,
    /// and what actually matters, is that the table reaches the DP rather than being stored and
    /// ignored -- so the same sanma-legal hand must come out with different numbers.
    #[test]
    fn sanma_copies_table_reaches_the_dp() {
        let mut solver = Solver::new();
        // Sanma-legal throughout: manzu only 1m/9m, no 5m.
        let hand = hand_from_mpsz("119m123456789p11s");
        let indicators = vec![Tile::from_str_checked("3z")];

        let yonma = SolverConfig::new(13, indicators.clone());
        let sanma = SolverConfig::new_in(108, sanma_copies(), sanma_dora_map(), 13, indicators);

        assert_eq!(yonma.sum, 136 - 13 - 1, "yonma wall size unchanged");
        assert_eq!(sanma.sum, 108 - 13 - 1, "sanma counts a 108-tile set");

        let unpack = |a: &Analysis| -> (Vec<(u8, u8)>, f64) {
            match a {
                Analysis::Full { stats, .. } => (
                    stats[0].necessary.clone(),
                    stats[0].win_prob.iter().cloned().fold(0.0, f64::max),
                ),
                other => panic!("expected the full DP, got {:?}", core::mem::discriminant(other)),
            }
        };
        let (nec_y, win_y) = unpack(&solver.analyze(&hand, &yonma));
        let (nec_s, win_s) = unpack(&solver.analyze(&hand, &sanma));

        // No phantom manzu is ever offered as an improving draw under the sanma table.
        assert!(
            nec_s.iter().all(|(k, w)| !(1..=7).contains(k) || *w == 0),
            "sanma offered phantom manzu: {:?}",
            nec_s
        );

        // The table is actually consulted: a smaller wall changes the draw probabilities.
        assert!(
            (win_y - win_s).abs() > 1e-9,
            "copies table / wall size never reached the DP: win_prob identical ({win_y})"
        );
        // ... and it is not merely `sum` doing the work -- yonma still counts 2m--8m as unseen.
        // ADR 0010's "28 phantom manzu" are 2m--8m x 4. In the 37-kind representation that is
        // 27 normal copies plus the red 5m, which lives at kind 34 rather than at kind 4 --
        // which is exactly why `sanma_copies` has to zero `RED_BASE` separately.
        let phantom_wall: u32 = (1..=7).map(|k| DEFAULT_COPIES[k] as u32).sum::<u32>()
            + DEFAULT_COPIES[RED_BASE] as u32;
        assert_eq!(phantom_wall, 28, "the 28 phantom manzu ADR 0010 names");
        assert_eq!(
            (1..=7).map(|k| sanma_copies()[k] as u32).sum::<u32>()
                + sanma_copies()[RED_BASE] as u32,
            0
        );
        let _ = nec_y;
    }

    /// The copies table must describe the tile set it claims to.
    /// A cap, not a count. Sanma's 55-draw live wall gives ~18 tsumos, so it loses at most one
    /// turn of lookahead on the first draw; raising this to 18 would change 4p solver output and
    /// break the encoder's byte-parity contract.
    #[test]
    fn max_tsumos_left_is_17_for_both_variants() {
        assert_eq!(MAX_TSUMOS_LEFT, 17);
    }

    #[test]
    fn sanma_copies_table_sums_to_the_wall() {
        assert_eq!(DEFAULT_COPIES.iter().map(|&c| c as u32).sum::<u32>(), DEFAULT_WALL_SIZE);
        assert_eq!(sanma_copies().iter().map(|&c| c as u32).sum::<u32>(), 108);
    }

    /// Own Kita is a flat han bonus, because the extracted North is no longer in the hand and
    /// no indicator arithmetic reproduces it.
    #[test]
    fn bonus_han_raises_the_score() {
        let mut solver = Solver::new();
        let hand = hand_from_mpsz("119m123456789p11s");
        let indicators = vec![Tile::from_str_checked("3z")];

        let base =
            SolverConfig::new_in(108, sanma_copies(), sanma_dora_map(), 13, indicators.clone());
        let with_kita = SolverConfig::new_in(108, sanma_copies(), sanma_dora_map(), 13, indicators)
            .with_bonus_han(3);

        assert_eq!(base.bonus_han, 0);
        assert_eq!(with_kita.bonus_han, 3);

        fn ev(solver: &mut Solver, hand: &TileSet37, cfg: &SolverConfig) -> f64 {
            match solver.analyze(hand, cfg) {
                Analysis::Full { stats, .. } => {
                    stats[0].exp_score.iter().cloned().fold(0.0, f64::max)
                }
                _ => panic!("expected the full DP"),
            }
        }
        let lo = ev(&mut solver, &hand, &base);
        let hi = ev(&mut solver, &hand, &with_kita);
        assert!(hi > lo, "bonus han did not raise EV: {} -> {}", lo, hi);
    }

    /// Yonma is untouched: the default table and dora map reproduce the previous hardcoded
    /// behaviour exactly. (The `goldens` integration tests are the byte-level version of this.)
    #[test]
    fn yonma_defaults_match_the_old_hardcoded_behaviour() {
        for k in 0..N_KINDS {
            let expected = match k {
                4 | 13 | 22 => 3,
                RED_BASE..=36 => 1,
                _ => 4,
            };
            assert_eq!(DEFAULT_COPIES[k], expected, "copies[{}]", k);
        }
        for k in 0..34u8 {
            let t = Tile::from_encoding(k).unwrap();
            assert_eq!(
                DEFAULT_DORA_MAP[k as usize],
                t.indicated_dora().normal_encoding(),
                "dora_map[{}]",
                k
            );
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
    fn min_tsumo_points_scores_complete_hands() {
        let mut solver = Solver::new();
        // Complete closed hand (tanki on 8s): menzen tsumo at minimum.
        let hand = hand_from_mpsz("234567m234567p88s");
        let cfg = SolverConfig::new(14, vec![Tile::from_str_checked("3z")]);
        let win = Tile::from_str_checked("8s");
        let pts = solver.min_tsumo_points(&hand, win, &cfg);
        assert!(pts > 0.0, "complete closed hand must score, got {pts}");
        // No uradora: the same call is deterministic in the indicators given.
        let pts2 = solver.min_tsumo_points(&hand, win, &cfg);
        assert_eq!(pts, pts2);
        // Incomplete hand: no agari, no points.
        let junk = hand_from_mpsz("1147m258p369s123z");
        assert_eq!(solver.min_tsumo_points(&junk, Tile::from_str_checked("1m"), &cfg), 0.0);
        // Winning tile absent from the hand.
        assert_eq!(solver.min_tsumo_points(&hand, Tile::from_str_checked("1z"), &cfg), 0.0);
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

    #[test]
    fn score_cache_is_bit_exact_and_reused() {
        clear_solver_caches();
        let hand = hand_from_mpsz("123456789m11p56s");
        let cfg = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        let mut solver = Solver::new();
        solver.cache_mode = CacheMode::Score;

        let first = solver.analyze(&hand, &cfg);
        let cached_entries = SCORE_CACHE.with(|cache| cache.borrow().values.len());
        assert!(cached_entries > 0, "tenpai analysis did not populate score cache");
        let second = solver.analyze(&hand, &cfg);

        assert_eq!(first, second);
        assert_eq!(
            SCORE_CACHE.with(|cache| cache.borrow().values.len()),
            cached_entries,
            "identical analysis should reuse score entries"
        );
    }

    #[test]
    fn warm_structure_is_exact_when_wall_weights_change() {
        let hand = hand_from_mpsz("123456789m11p56s");
        let base = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        let mut changed = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        changed.t_max = 9;
        changed.copies[21] = 0; // 4s is a structural wait, but absent from this wall.

        clear_solver_caches();
        let expected = Solver::new().analyze(&hand, &changed);

        clear_solver_caches();
        let mut solver = Solver::new();
        solver.cache_mode = CacheMode::Leaf;
        let _ = solver.analyze(&hand, &base);
        let cached_nodes = STRUCTURE_CACHE.with(|cache| {
            let cache = cache.borrow();
            cache.n13.len() + cache.n14.len()
        });
        let actual = solver.analyze(&hand, &changed);

        assert_eq!(actual, expected);
        assert_eq!(
            STRUCTURE_CACHE.with(|cache| {
                let cache = cache.borrow();
                cache.n13.len() + cache.n14.len()
            }),
            cached_nodes,
            "same root should reuse all cached node structure"
        );
    }

    #[test]
    fn topology_arena_is_exact_and_reused() {
        let hand = hand_from_mpsz("123456789m11p56s");
        let base = SolverConfig::new(13, vec![Tile::from_str_checked("3p")]);
        let mut changed = base.clone();
        changed.t_max = 9;
        changed.copies[21] = 0;

        let mut uncached = Solver::new();
        uncached.cache_mode = CacheMode::None;
        let expected = uncached.analyze(&hand, &changed);

        let mut arena = Solver::new();
        arena.cache_mode = CacheMode::Arena;
        let first = arena.analyze(&hand, &base);
        let actual = arena.analyze(&hand, &changed);
        let nodes = arena.topology.n13.len() + arena.topology.n14.len();
        let repeated = arena.analyze(&hand, &base);

        assert_eq!(actual, expected);
        assert_eq!(repeated, first);
        assert_eq!(arena.topology.n13.len() + arena.topology.n14.len(), nodes);
    }
}
