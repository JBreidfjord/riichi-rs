use crate::player::Player;

/// Points / point differences. Usually increments of 100.
pub type GamePoints = i64;

/// Wind index --- 0, 1, 2, 3 => east, south, west, north.
///
/// Note that this is _identical_ to [`Player`] --- see its definition.
pub type Wind = Player;

/// Catch-all error for cases where details of the error are unnecessary.
#[derive(Debug)]
pub struct UnspecifiedError;

#[cfg(feature = "std")]
impl std::error::Error for UnspecifiedError {}

impl core::fmt::Display for UnspecifiedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Unspecified error from riichi-elements.")
    }
}
