use super::Yaku::{self, *};

/// There are pairs of [`Yaku`]'s that will not be awarded to the same hand even when conditions
/// for both are satisfied; only the higher priority Yaku will be awarded. Given a Yaku, returns
/// all the _non-trivially_ (see below) conflicting Yaku('s) with a _lower_ priority.
///
/// By non-trivial we mean:
///
/// - There exist a winning hand for which both the higher and lower priority Yakus' conditions
///   can be satisfied.
///
/// - The Yaku's are not obviously variants of each other, e.g.:
///   - [`DoubleRiichi`] over [`Riichi`].
///   - [`Junchantaiyaochuu`] over [`Honchantaiyaochuu`];
///     basically all the "Jun"/"Chin" (without honors) over "Hon" (with honors) pairs.
///   - [`SuuankouTanki`] over [`Suuankou`].
///
/// There are actually only a few cases; see implementation for details.
///
pub const fn get_blocked_yaku(yaku: Yaku) -> &'static [Yaku] {
    match yaku {
        // "terminals only" imply "each group/pair contains a terminal"
        Chinroutou | Honroutou => &[Junchantaiyaochuu, Honchantaiyaochuu],

        // Kan draw (from the dead wall) is _not_ considered "Haitei"/"Houtei" (last chance).
        Rinshankaihou | Chankan => &[Haiteimouyue, Houteiraoyui],
        _ => &[],
    }
}
