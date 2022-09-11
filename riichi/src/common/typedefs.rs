use super::Player;

/// Points / point differences. Usually increments of 100.
pub type GamePoints = i64;

/// Wind index --- 0, 1, 2, 3 => east, south, west, north.
///
/// Note that this is _identical_ to [`Player`] --- see its definition.
pub type Wind = Player;
