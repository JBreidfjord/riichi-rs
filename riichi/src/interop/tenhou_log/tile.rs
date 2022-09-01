use crate::common::*;
use crate::utils::*;

pub fn parse_tenhou_tile(tt: u8) -> Option<Tile> {
    match tt {
        11..=19 => Tile::from_encoding(0 + (tt - 11)),
        21..=29 => Tile::from_encoding(9 + (tt - 21)),
        31..=39 => Tile::from_encoding(18 + (tt - 31)),
        41..=47 => Tile::from_encoding(27 + (tt - 41)),
        51..=53 => Tile::from_encoding(34 + (tt - 51)),
        _ => None,
    }
}

pub fn to_tenhou_tile(tile: Tile) -> u8 {
    if tile.is_red() {
        tile.encoding() - 34 + 51
    } else {
        tile.num() + (tile.suit() + 1) * 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn tenhou_tiles() {
        let tts = [
            11, 12, 13, 14, 15, 16, 17, 18, 19,
            21, 22, 23, 24, 25, 26, 27, 28, 29,
            31, 32, 33, 34, 35, 36, 37, 38, 39,
            41, 42, 43, 44, 45, 46, 47,
            51, 52, 53,
        ];
        for (enc, tt) in tts.into_iter().enumerate() {
            let tile = Tile::from_encoding(enc as u8).unwrap();
            let parsed = parse_tenhou_tile(tt).unwrap();
            let serialized = to_tenhou_tile(tile);
            assert_eq!(parsed, tile);
            assert_eq!(serialized, tt);
        }
    }
}
