//! Shanten number calculation.
//!
//! The shanten number of a hand is the minimum number of tile exchanges needed
//! to reach tenpai; 0 means tenpai, -1 means the hand is complete.

use riichi_elements::prelude::*;

/// Shanten number of a closed hand (reds folded into normal tiles).
///
/// The hand must hold 3N+1 or 3N+2 tiles (N = 0..=4); melds are implied by the
/// missing tiles, e.g. a 10-tile hand needs 3 more groups + the pair. Irregular
/// forms (seven pairs, thirteen orphans) are only considered for meld-free
/// hands (13 or 14 tiles).
///
/// Returns 0 for tenpai, -1 for a complete 3N+2 hand.
pub fn shanten(hand: &TileSet34) -> i8 {
    let total: usize = hand.0.iter().map(|&c| c as usize).sum();
    debug_assert!(
        total <= 14 && matches!(total % 3, 1 | 2),
        "hand must hold 3N+1 or 3N+2 tiles, got {total}"
    );
    let groups = (total / 3) as i8;

    let mut counts = hand.0;
    let mut best = i8::MAX;
    dfs_sets(&mut counts, 0, 0, groups, &mut best);

    if total >= 13 {
        best = best.min(chiitoi_shanten(&hand.0)).min(kokushi_shanten(&hand.0));
    }
    best
}

/// Phase 1: peel off complete groups (triplets and runs) starting at index `i`,
/// then hand the remainder to the partial-group phase.
fn dfs_sets(counts: &mut [u8; 34], mut i: usize, sets: i8, groups: i8, best: &mut i8) {
    while i < 34 && counts[i] == 0 {
        i += 1;
    }
    if i == 34 || sets == groups {
        dfs_partials(counts, 0, sets, 0, false, groups, best);
        return;
    }

    if counts[i] >= 3 {
        counts[i] -= 3;
        dfs_sets(counts, i, sets + 1, groups, best);
        counts[i] += 3;
    }
    if i < 27 && i % 9 <= 6 && counts[i + 1] > 0 && counts[i + 2] > 0 {
        counts[i] -= 1;
        counts[i + 1] -= 1;
        counts[i + 2] -= 1;
        dfs_sets(counts, i, sets + 1, groups, best);
        counts[i] += 1;
        counts[i + 1] += 1;
        counts[i + 2] += 1;
    }
    // Leave every remaining copy of tile `i` to the partial phase.
    dfs_sets(counts, i + 1, sets, groups, best);
}

/// Phase 2: pick partial groups (pair, adjacent, gapped) and at most one pair
/// as the head, then score: shanten = 2*(groups - sets) - partials - head.
fn dfs_partials(
    counts: &mut [u8; 34],
    mut i: usize,
    sets: i8,
    partials: i8,
    has_head: bool,
    groups: i8,
    best: &mut i8,
) {
    while i < 34 && counts[i] == 0 {
        i += 1;
    }
    if i == 34 {
        let score = 2 * (groups - sets) - partials - i8::from(has_head);
        *best = (*best).min(score);
        return;
    }

    if sets + partials < groups {
        if counts[i] >= 2 {
            counts[i] -= 2;
            dfs_partials(counts, i, sets, partials + 1, has_head, groups, best);
            counts[i] += 2;
        }
        if i < 27 && i % 9 <= 7 && counts[i + 1] > 0 {
            counts[i] -= 1;
            counts[i + 1] -= 1;
            dfs_partials(counts, i, sets, partials + 1, has_head, groups, best);
            counts[i] += 1;
            counts[i + 1] += 1;
        }
        if i < 27 && i % 9 <= 6 && counts[i + 2] > 0 {
            counts[i] -= 1;
            counts[i + 2] -= 1;
            dfs_partials(counts, i, sets, partials + 1, has_head, groups, best);
            counts[i] += 1;
            counts[i + 2] += 1;
        }
    }
    if !has_head && counts[i] >= 2 {
        counts[i] -= 2;
        dfs_partials(counts, i, sets, partials, true, groups, best);
        counts[i] += 2;
    }
    dfs_partials(counts, i + 1, sets, partials, has_head, groups, best);
}

/// Seven-pairs shanten: 6 - pairs, plus a penalty when there are fewer than 7
/// kinds (chiitoi needs 7 distinct pairs; extra copies beyond 2 are dead).
fn chiitoi_shanten(counts: &[u8; 34]) -> i8 {
    let pairs = counts.iter().filter(|&&c| c >= 2).count() as i8;
    let kinds = counts.iter().filter(|&&c| c >= 1).count() as i8;
    6 - pairs + (7 - kinds).max(0)
}

/// Thirteen-orphans shanten: 13 - orphan kinds - (has any orphan pair).
fn kokushi_shanten(counts: &[u8; 34]) -> i8 {
    const ORPHANS: [usize; 13] = [0, 8, 9, 17, 18, 26, 27, 28, 29, 30, 31, 32, 33];
    let kinds = ORPHANS.iter().filter(|&&t| counts[t] >= 1).count() as i8;
    let has_pair = ORPHANS.iter().any(|&t| counts[t] >= 2);
    13 - kinds - i8::from(has_pair)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decomposer, WaitSet};

    fn hand(s: &str) -> TileSet34 {
        let set: TileSet37 = tiles_from_str(s).collect();
        TileSet34::from(&set)
    }

    #[test]
    fn complete_hand_is_minus_one() {
        assert_eq!(shanten(&hand("123456789m11122z")), -1);
        assert_eq!(shanten(&hand("11223344556677z")), -1); // chiitoi complete
    }

    #[test]
    fn tanki_tenpai_is_zero() {
        assert_eq!(shanten(&hand("123456789m1112z")), 0);
    }

    #[test]
    fn shanpon_tenpai_is_zero() {
        // 123m 456m 789s complete; 11p/22s shanpon
        assert_eq!(shanten(&hand("123456m11p22s789s")), 0);
    }

    #[test]
    fn kokushi_thirteen_wait_is_zero() {
        assert_eq!(shanten(&hand("19m19p19s1234567z")), 0);
    }

    #[test]
    fn chiitoi_tenpai_is_zero() {
        assert_eq!(shanten(&hand("1199m1199p1199s7z")), 0);
    }

    #[test]
    fn one_shanten_regular() {
        // 123m 456m 789s + 11p head; 2s5s disconnected -> 1-shanten
        assert_eq!(shanten(&hand("123456m11p25s789s")), 1);
    }

    #[test]
    fn three_shanten_regular() {
        // 555z set; kanchans 13m 24p 68p; no pair -> 3-shanten
        assert_eq!(shanten(&hand("1359m2468p13s555z2z")), 3);
    }

    #[test]
    fn worst_hand_is_six_shanten() {
        // Max shanten for a 13-tile hand is 6 (via chiitoi: 13 distinct kinds)
        assert_eq!(shanten(&hand("147m258p369s1234z")), 6);
    }

    #[test]
    fn meld_hand_tenpai() {
        // 10 tiles = one meld out: 123m 456p sets, 11s head, 89s waiting 7s
        assert_eq!(shanten(&hand("123m456p11s89s")), 0);
    }

    #[test]
    fn meld_hand_one_shanten() {
        // 7 tiles = two melds out: 123m set, 55p head, 2z9s junk -> 1-shanten
        assert_eq!(shanten(&hand("123m55p9s2z")), 1);
    }

    #[test]
    fn duplicate_runs_beat_pair_hoarding() {
        // Best decomposition reuses doubled tiles as two runs: 123m 123m 555m
        // sets + 44m head (5m, 9s float) -> 2*(4-3) - 0 - 1 = 1.
        // Chiitoi is worse: 5 pairs, 6 kinds -> 6-5+1 = 2.
        assert_eq!(shanten(&hand("11223344m5555m9s")), 1);
    }

    /// Deterministic LCG so the property tests are reproducible.
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

    /// shanten == 0 must agree with the decomposer's wait set, except for
    /// junkara hands: structurally tenpai but every winning tile is already
    /// held 4 times, so no live wait exists. Shanten is structural (Tenhou
    /// counts junkara as tenpai); the wait set only lists winnable tiles.
    fn assert_tenpai_consistent(decomposer: &mut Decomposer, h: &TileSet34) -> bool {
        let s = shanten(h);
        let waits = WaitSet::from_tile_set(decomposer, h);
        if s == 0 && !waits.waiting_tiles.any() {
            for t in 0..34 {
                if h.0[t] < 4 {
                    let mut c = h.0;
                    c[t] += 1;
                    assert_ne!(
                        shanten(&TileSet34(c)),
                        -1,
                        "live tile completes the hand but wait set was empty: {h}"
                    );
                }
            }
        } else {
            assert_eq!(
                s == 0,
                waits.waiting_tiles.any(),
                "shanten={s} but waits={} for {h}",
                waits.waiting_tiles
            );
        }
        s == 0
    }

    #[test]
    fn tenpai_iff_waitset_nonempty() {
        let mut decomposer = Decomposer::new();
        let mut rng = Lcg(0xC17A05);
        let mut tenpai_seen = 0;
        for _ in 0..500 {
            let h = random_hand(&mut rng, 13);
            assert!(shanten(&h) >= 0, "13-tile hand cannot be complete: {h}");
            if assert_tenpai_consistent(&mut decomposer, &h) {
                tenpai_seen += 1;
            }
        }
        // Purely random tenpai is rare; seed biased hands to exercise the branch too.
        let mut rng = Lcg(0x5EED);
        for _ in 0..300 {
            // Build a near-complete hand: 3 runs + triplet + one random tile
            let mut counts = [0u8; 34];
            for _ in 0..3 {
                let suit = rng.next(3);
                let start = rng.next(7);
                for d in 0..3 {
                    let t = suit * 9 + start + d;
                    if counts[t] < 4 {
                        counts[t] += 1;
                    }
                }
            }
            let trip = rng.next(34);
            counts[trip] = (counts[trip] + 3).min(4);
            let mut placed: usize = counts.iter().map(|&c| c as usize).sum();
            while placed < 13 {
                let t = rng.next(34);
                if counts[t] < 4 {
                    counts[t] += 1;
                    placed += 1;
                }
            }
            let h = TileSet34(counts);
            if assert_tenpai_consistent(&mut decomposer, &h) {
                tenpai_seen += 1;
            }
        }
        assert!(tenpai_seen > 10, "property test never exercised tenpai hands");
    }

    #[test]
    fn fourteen_equals_min_over_discards() {
        let mut rng = Lcg(0xFEED);
        for _ in 0..200 {
            let h14 = random_hand(&mut rng, 14);
            let s14 = shanten(&h14);
            let mut min13 = i8::MAX;
            for t in 0..34 {
                if h14.0[t] == 0 {
                    continue;
                }
                let mut c = h14.0;
                c[t] -= 1;
                min13 = min13.min(shanten(&TileSet34(c)));
            }
            assert_eq!(
                s14, min13,
                "14-tile shanten must equal best discard's 13-tile shanten for {h14}"
            );
        }
    }

    #[test]
    fn waiting_tile_completes_the_hand() {
        let mut decomposer = Decomposer::new();
        // Tenpai hand: drawing any waiting tile must give shanten -1
        let h = hand("123456789m1112z");
        let waits = WaitSet::from_tile_set(&mut decomposer, &h);
        assert!(waits.waiting_tiles.any());
        for t in 0..34u8 {
            if !waits.waiting_tiles.has_i(t) {
                continue;
            }
            let mut c = h.0;
            c[t as usize] += 1;
            assert_eq!(shanten(&TileSet34(c)), -1);
        }
    }
}
