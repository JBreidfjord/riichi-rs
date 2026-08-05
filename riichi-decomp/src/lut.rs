//! Table-based shanten with set-valued advancing/keep-discard calculators.
//!
//! Replaces per-tile `shanten()` probing (37 draw + 14 discard calls per
//! solver vertex) with O(1) lookups into precomputed per-suit distance
//! tables, derived clean-room from the replacement-distance formulation:
//!
//!   d_suit(h, m, p) = min additions to complete m groups + p pairs within
//!   the suit; shanten = (min over suit assignments of Σ d) - 1.
//!
//! Tables are generated on first use (a few seconds) and cached to a
//! versioned file next to the running binary.

use std::sync::OnceLock;

use riichi_elements::prelude::*;

const NUM_DIGITS: usize = 9;
const HONOR_DIGITS: usize = 7;
const NUM_SIZE: usize = 5usize.pow(NUM_DIGITS as u32); // 1_953_125
const HONOR_SIZE: usize = 5usize.pow(HONOR_DIGITS as u32); // 78_125
/// Targets per suit index: melds 0..=4 x pair 0..=1, laid out m*2+p.
const TARGETS: usize = 10;
const INF: u8 = 100;

const ORPHANS: [usize; 13] = [0, 8, 9, 17, 18, 26, 27, 28, 29, 30, 31, 32, 33];

const CACHE_MAGIC: &[u8; 8] = b"RDLUT02\0";

pub struct ShantenLut {
    /// Shared by the three number suits: NUM_SIZE * TARGETS.
    num: Box<[u8]>,
    /// HONOR_SIZE * TARGETS.
    honor: Box<[u8]>,
}

static LUT: OnceLock<ShantenLut> = OnceLock::new();

impl ShantenLut {
    /// The process-wide table set; generates + caches to disk on first use.
    pub fn get() -> &'static ShantenLut {
        LUT.get_or_init(|| {
            let path = cache_path();
            if let Some(p) = &path {
                if let Some(lut) = ShantenLut::load(p) {
                    return lut;
                }
            }
            let lut = ShantenLut::generate();
            if let Some(p) = &path {
                let _ = lut.save(p);
            }
            lut
        })
    }

    /// Shanten number; agrees exactly with [`crate::shanten`].
    pub fn shanten(&self, hand: &TileSet34) -> i8 {
        let q = Query::new(self, &hand.0);
        q.shanten()
    }

    /// For a 3N+1 hand: (shanten, mask of tile kinds whose draw lowers it).
    pub fn analyze_13(&self, hand: &TileSet34) -> (i8, u64) {
        let q = Query::new(self, &hand.0);
        let best = q.shanten();
        let mut mask = 0u64;

        if q.std == best {
            q.for_optimal_targets(|suit, tgt, val| {
                let (base, digits, table, idx, p5) = q.suit_view(suit);
                for (i, &d) in digits.iter().enumerate() {
                    if d < 4 && table[(idx + p5[i]) * TARGETS + tgt] + 1 == val {
                        mask |= 1 << (base + i);
                    }
                }
            });
        }
        if q.total >= 13 {
            if q.chiitoi == best {
                for t in 0..34 {
                    match q.h[t] {
                        1 => mask |= 1 << t,
                        0 if q.kinds < 7 => mask |= 1 << t,
                        _ => {}
                    }
                }
            }
            if q.kokushi == best {
                for &t in &ORPHANS {
                    if q.h[t] == 0 || !q.orphan_pair {
                        mask |= 1 << t;
                    }
                }
            }
        }
        (best, mask)
    }

    /// For a 3N+2 hand: (shanten, mask of held kinds whose discard keeps it).
    /// A complete hand (-1) returns an empty mask.
    pub fn analyze_14(&self, hand: &TileSet34) -> (i8, u64) {
        let q = Query::new(self, &hand.0);
        let best = q.shanten();
        if best == -1 {
            return (-1, 0);
        }
        let mut mask = 0u64;

        if q.std == best {
            q.for_optimal_targets(|suit, tgt, val| {
                let (base, digits, table, idx, p5) = q.suit_view(suit);
                for (i, &d) in digits.iter().enumerate() {
                    if d > 0 && table[(idx - p5[i]) * TARGETS + tgt] == val {
                        mask |= 1 << (base + i);
                    }
                }
            });
        }
        if q.total >= 13 {
            if q.chiitoi == best {
                for t in 0..34 {
                    match q.h[t] {
                        c if c >= 3 => mask |= 1 << t,
                        1 if q.kinds > 7 => mask |= 1 << t,
                        _ => {}
                    }
                }
            }
            if q.kokushi == best {
                let orphan_pairs = ORPHANS.iter().filter(|&&t| q.h[t] >= 2).count();
                for t in 0..34 {
                    if q.h[t] == 0 {
                        continue;
                    }
                    let keep = if !ORPHANS.contains(&t) {
                        true
                    } else {
                        q.h[t] >= 3 || (q.h[t] == 2 && orphan_pairs >= 2)
                    };
                    if keep {
                        mask |= 1 << t;
                    }
                }
            }
        }
        (best, mask)
    }
}

/// Per-hand query state: suit indices, per-suit target rows, combine tables.
struct Query<'a> {
    lut: &'a ShantenLut,
    h: [u8; 34],
    total: u8,
    groups: usize,
    /// Suit index (base-5 digit vector) per suit: m, p, s, z.
    idx: [usize; 4],
    /// Per-suit target rows: t[suit][m*2+p] = d_suit.
    t: [[u8; TARGETS]; 4],
    /// f[s] = min cost combining suits 0..s; b[s] = combining suits s..4.
    /// Indexed [melds][pair].
    f: [[[u8; 2]; 5]; 5],
    b: [[[u8; 2]; 5]; 5],
    std: i8,
    chiitoi: i8,
    kokushi: i8,
    kinds: i8,
    orphan_pair: bool,
}

const P5: [usize; 9] = {
    let mut p = [0usize; 9];
    let mut i = 0;
    let mut v = 1;
    while i < 9 {
        p[i] = v;
        v *= 5;
        i += 1;
    }
    p
};

const IDENT: [[u8; 2]; 5] = {
    let mut t = [[INF; 2]; 5];
    t[0][0] = 0;
    t
};

/// C[m'][p'] = min over (dm, dp) of A[m'-dm][p'-dp] + row[dm*2+dp].
fn combine(a: &[[u8; 2]; 5], row: &[u8; TARGETS]) -> [[u8; 2]; 5] {
    let mut c = [[INF; 2]; 5];
    for m1 in 0..5 {
        for p1 in 0..2 {
            let base = a[m1][p1];
            if base >= INF {
                continue;
            }
            for dm in 0..5 - m1 {
                for dp in 0..2 - p1 {
                    let v = base.saturating_add(row[dm * 2 + dp]);
                    if v < c[m1 + dm][p1 + dp] {
                        c[m1 + dm][p1 + dp] = v;
                    }
                }
            }
        }
    }
    c
}

impl<'a> Query<'a> {
    fn new(lut: &'a ShantenLut, h: &[u8; 34]) -> Query<'a> {
        let total: u8 = h.iter().sum();
        debug_assert!(
            total <= 14 && matches!(total % 3, 1 | 2),
            "hand must hold 3N+1 or 3N+2 tiles, got {total}"
        );
        let groups = (total / 3) as usize;

        let mut idx = [0usize; 4];
        for s in 0..3 {
            for i in 0..NUM_DIGITS {
                idx[s] += h[s * 9 + i] as usize * P5[i];
            }
        }
        for i in 0..HONOR_DIGITS {
            idx[3] += h[27 + i] as usize * P5[i];
        }

        let mut t = [[0u8; TARGETS]; 4];
        for s in 0..4 {
            let table = if s < 3 { &lut.num } else { &lut.honor };
            t[s].copy_from_slice(&table[idx[s] * TARGETS..idx[s] * TARGETS + TARGETS]);
        }

        let mut f = [IDENT; 5];
        for s in 0..4 {
            f[s + 1] = combine(&f[s], &t[s]);
        }
        let mut b = [IDENT; 5];
        for s in (0..4).rev() {
            b[s] = combine(&b[s + 1], &t[s]);
        }

        let std = f[4][groups][1] as i8 - 1;

        let (chiitoi, kokushi, kinds, orphan_pair) = if total >= 13 {
            let pairs = h.iter().filter(|&&c| c >= 2).count() as i8;
            let kinds = h.iter().filter(|&&c| c >= 1).count() as i8;
            let ok = ORPHANS.iter().filter(|&&t| h[t] >= 1).count() as i8;
            let op = ORPHANS.iter().any(|&t| h[t] >= 2);
            (6 - pairs + (7 - kinds).max(0), 13 - ok - i8::from(op), kinds, op)
        } else {
            (i8::MAX, i8::MAX, 0, false)
        };

        Query {
            lut,
            h: *h,
            total,
            groups,
            idx,
            t,
            f,
            b,
            std,
            chiitoi,
            kokushi,
            kinds,
            orphan_pair,
        }
    }

    fn shanten(&self) -> i8 {
        self.std.min(self.chiitoi).min(self.kokushi)
    }

    /// (tile-index base, digit count, table, suit index, powers) for a suit.
    fn suit_view(&self, suit: usize) -> (usize, &[u8], &[u8], usize, &[usize]) {
        if suit < 3 {
            (
                suit * 9,
                &self.h[suit * 9..suit * 9 + 9],
                &self.lut.num,
                self.idx[suit],
                &P5[..NUM_DIGITS],
            )
        } else {
            (27, &self.h[27..34], &self.lut.honor, self.idx[3], &P5[..HONOR_DIGITS])
        }
    }

    /// Visit each (suit, target, d_suit value) that lies on at least one
    /// optimal suit assignment for the standard-form distance.
    fn for_optimal_targets(&self, mut visit: impl FnMut(usize, usize, u8)) {
        let d = self.f[4][self.groups][1];
        for s in 0..4 {
            let mut seen = [false; TARGETS];
            for m1 in 0..=self.groups {
                for p1 in 0..2 {
                    let pre = self.f[s][m1][p1];
                    if pre >= INF {
                        continue;
                    }
                    for dm in 0..=self.groups - m1 {
                        for dp in 0..2 - p1 {
                            let tgt = dm * 2 + dp;
                            if seen[tgt] {
                                continue;
                            }
                            let post = self.b[s + 1][self.groups - m1 - dm][1 - p1 - dp];
                            if post >= INF {
                                continue;
                            }
                            if pre + self.t[s][tgt] + post == d {
                                seen[tgt] = true;
                                visit(s, tgt, self.t[s][tgt]);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Table generation
// ---------------------------------------------------------------------------

impl ShantenLut {
    /// Generate both tables from scratch (a few seconds in release).
    pub fn generate() -> ShantenLut {
        ShantenLut {
            num: gen_suit_table(NUM_DIGITS, true),
            honor: gen_suit_table(HONOR_DIGITS, false),
        }
    }

    fn load(path: &std::path::Path) -> Option<ShantenLut> {
        let bytes = std::fs::read(path).ok()?;
        let expect = CACHE_MAGIC.len() + (NUM_SIZE + HONOR_SIZE) * TARGETS;
        if bytes.len() != expect || &bytes[..CACHE_MAGIC.len()] != CACHE_MAGIC {
            return None;
        }
        let num_len = NUM_SIZE * TARGETS;
        let body = &bytes[CACHE_MAGIC.len()..];
        Some(ShantenLut {
            num: body[..num_len].into(),
            honor: body[num_len..].into(),
        })
    }

    fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let tmp = path.with_extension("tmp");
        let mut bytes = Vec::with_capacity(CACHE_MAGIC.len() + self.num.len() + self.honor.len());
        bytes.extend_from_slice(CACHE_MAGIC);
        bytes.extend_from_slice(&self.num);
        bytes.extend_from_slice(&self.honor);
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, path)
    }
}

fn cache_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("shanten-lut-v1.bin"))
}

/// Distance table for one suit: for every base-5 count vector, the minimum
/// number of tile additions to complete m groups + p pairs (m*2+p layout).
///
/// Goals are NOT capped at 4 copies per kind: standard shanten is structural
/// (a tanki on a kind whose four copies are all in hand still counts as
/// tenpai — the "junkara" convention, matching the DFS in `shanten.rs`), so
/// an addition may hypothetically be a 5th copy.
///
/// DP over positions with run-carry state: entering position `pos`, `(a, b)`
/// = runs started at pos-2 / pos-1 (each consumes one tile here; `b` also
/// consumes one at pos+1). Choices at pos: `t` triplets, `r` new runs, `pr`
/// pair. Tiles used = 3t + 2pr + a + b + r; additions = max(0, used - held).
/// The digit DFS shares prefix layers across all suit vectors.
fn gen_suit_table(digits: usize, runs: bool) -> Box<[u8]> {
    let size = 5usize.pow(digits as u32);
    let mut table = vec![INF; size * TARGETS].into_boxed_slice();

    // layer[a][b][g][p] = min additions so far (INF = unreachable).
    type Layer = [[[[u8; 2]; 5]; 5]; 5];
    let mut init: Layer = [[[[INF; 2]; 5]; 5]; 5];
    init[0][0][0][0] = 0;

    fn rec(table: &mut [u8], digits: usize, runs: bool, pos: usize, idx: usize, layer: &Layer) {
        if pos == digits {
            for g in 0..5 {
                for p in 0..2 {
                    table[idx * TARGETS + g * 2 + p] = layer[0][0][g][p];
                }
            }
            return;
        }
        let r_allowed = runs && pos + 2 < digits;
        for digit in 0..5u8 {
            let mut next: Layer = [[[[INF; 2]; 5]; 5]; 5];
            let mut any = false;
            for a in 0..5usize {
                for b in 0..5 - a {
                    for g in (a + b)..5 {
                        for p in 0..2usize {
                            let c = layer[a][b][g][p];
                            if c >= INF {
                                continue;
                            }
                            for t in 0..5 - g {
                                let rmax = if r_allowed { 4 - g - t } else { 0 };
                                for r in 0..=rmax {
                                    for pr in 0..2 - p {
                                        let used = (3 * t + 2 * pr + a + b + r) as u8;
                                        let nc = c + used.saturating_sub(digit);
                                        // Suit distances never exceed 14
                                        // (a full goal from nothing).
                                        if nc > 14 {
                                            continue;
                                        }
                                        let slot = &mut next[b][r][g + t + r][p + pr];
                                        if nc < *slot {
                                            *slot = nc;
                                            any = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if any {
                rec(table, digits, runs, pos + 1, idx + digit as usize * P5[pos], &next);
            }
        }
    }

    rec(&mut table, digits, runs, 0, 0, &init);
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shanten;

    fn hand(s: &str) -> TileSet34 {
        let set: TileSet37 = tiles_from_str(s).collect();
        TileSet34::from(&set)
    }

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, bound: usize) -> usize {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 33) as usize) % bound
        }
    }

    fn random_hand(rng: &mut Lcg, tiles: usize) -> TileSet34 {
        let mut counts = [0u8; 34];
        let mut placed = 0;
        while placed < tiles {
            let t = rng.next(34);
            if counts[t] < 4 {
                counts[t] += 1;
                placed += 1;
            }
        }
        TileSet34(counts)
    }

    #[test]
    fn matches_dfs_on_fixed_hands() {
        let lut = ShantenLut::get();
        for s in [
            "123456789m11122z",
            "11223344556677z",
            "123456789m1112z",
            "123456m11p22s789s",
            "19m19p19s1234567z",
            "1199m1199p1199s7z",
            "123456m11p25s789s",
            "1359m2468p13s555z2z",
            "147m258p369s1234z",
            "123m456p11s89s",
            "123m55p9s2z",
            "11223344m5555m9s",
        ] {
            let h = hand(s);
            assert_eq!(lut.shanten(&h), shanten(&h), "hand {s}");
        }
    }

    #[test]
    fn matches_dfs_on_random_hands() {
        let lut = ShantenLut::get();
        let mut rng = Lcg(0xD15C0);
        for tiles in [13usize, 14, 10, 11, 7, 8, 4, 5, 1, 2] {
            let n = if tiles >= 13 { 20_000 } else { 4_000 };
            for _ in 0..n {
                let h = random_hand(&mut rng, tiles);
                assert_eq!(lut.shanten(&h), shanten(&h), "hand {h}");
            }
        }
    }

    #[test]
    fn analyze_13_matches_probing() {
        let lut = ShantenLut::get();
        let mut rng = Lcg(0xA113);
        for tiles in [13usize, 10, 7, 4, 1] {
            for _ in 0..3_000 {
                let h = random_hand(&mut rng, tiles);
                let (s, mask) = lut.analyze_13(&h);
                assert_eq!(s, shanten(&h), "hand {h}");
                for t in 0..34 {
                    let expect = if h.0[t] < 4 {
                        let mut c = h.0;
                        c[t] += 1;
                        shanten(&TileSet34(c)) < s
                    } else {
                        false
                    };
                    assert_eq!(
                        mask & (1 << t) != 0,
                        expect,
                        "hand {h} tile {t}: advancing mismatch (shanten {s})"
                    );
                }
            }
        }
    }

    #[test]
    fn analyze_14_matches_probing() {
        let lut = ShantenLut::get();
        let mut rng = Lcg(0x14141);
        for tiles in [14usize, 11, 8, 5, 2] {
            for _ in 0..3_000 {
                let h = random_hand(&mut rng, tiles);
                let (s, mask) = lut.analyze_14(&h);
                assert_eq!(s, shanten(&h), "hand {h}");
                if s == -1 {
                    assert_eq!(mask, 0);
                    continue;
                }
                for t in 0..34 {
                    let expect = if h.0[t] > 0 {
                        let mut c = h.0;
                        c[t] -= 1;
                        shanten(&TileSet34(c)) == s
                    } else {
                        false
                    };
                    assert_eq!(
                        mask & (1 << t) != 0,
                        expect,
                        "hand {h} tile {t}: keep-discard mismatch (shanten {s})"
                    );
                }
            }
        }
    }

    #[test]
    fn cache_roundtrip() {
        let lut = ShantenLut::generate();
        let dir = std::env::temp_dir().join("riichi-decomp-lut-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shanten-lut-v1.bin");
        lut.save(&path).unwrap();
        let loaded = ShantenLut::load(&path).unwrap();
        assert_eq!(&lut.num[..], &loaded.num[..]);
        assert_eq!(&lut.honor[..], &loaded.honor[..]);
    }
}
