
use once_cell::sync::OnceCell;
use regex::Regex;

use crate::common::*;
use crate::utils::*;
use super::tile::*;

pub fn parse_tenhou_meld(s: &str) -> Option<Meld> {
    static RE: OnceCell<Regex> = OnceCell::new();
    let re = RE.get_or_init(|| Regex::new(r"(?x)
      ([cpmk])?(\d\d)  # 1 2 chii/pon/daiminkan/kakan
       ([pmk])?(\d\d)? # 3 4 pon/daiminkan/kakan
        ([pk])?(\d\d)? # 5 6 pon/kakan
        ([ma])?(\d\d)? # 7 8 daiminkan/ankan
    ").unwrap());
    let groups = re.captures(s)?;
    let (dir, mode_str) =
        if let Some(m) = groups.get(1) { (3, m.as_str()) }
        else if let Some(m) = groups.get(3) { (2, m.as_str()) }
        else if let Some(m) = groups.get(5) { (1, m.as_str()) }
        else if let Some(m) = groups.get(7) { (1, m.as_str()) }  // not typo
        else { return None; };
    // tiles as appeared in the str
    let [a, b, c, d] = [2, 4, 6, 8].map(|i|
        groups.get(i)
            .and_then(|g| g.as_str().parse::<u8>().ok())
            .and_then(parse_tenhou_tile)
    );
    match mode_str {
        "c" => {
            Chii::from_tiles(b?, c?, a?)
                .map(|chii| Meld::Chii(chii))
        }
        "p" => {
            let dir_p = Player::new(dir);
            match dir {
                1 => Pon::from_tiles_dir(a?, b?, c?, dir_p),
                2 => Pon::from_tiles_dir(a?, c?, b?, dir_p),
                3 => Pon::from_tiles_dir(c?, a?, b?, dir_p),
                _ => panic!(),
            }.map(|pon| Meld::Pon(pon))
        }
        "k" => {
            let dir_p = Player::new(dir);
            match dir {
                1 => Pon::from_tiles_dir(a?, b?, d?, dir_p)
                    .and_then(|pon| Kakan::from_pon_added(pon, c?)),
                2 => Pon::from_tiles_dir(a?, c?, d?, dir_p)
                    .and_then(|pon| Kakan::from_pon_added(pon, b?)),
                3 => Pon::from_tiles_dir(b?, c?, d?, dir_p)
                    .and_then(|pon| Kakan::from_pon_added(pon, a?)),
                _ => panic!(),
            }.map(|kakan| Meld::Kakan(kakan))
        }
        "m" => {
            let dir_p = Player::new(dir);
            match dir {
                1 => Daiminkan::from_tiles_dir([a?, b?, c?], d?, dir_p),
                2 => Daiminkan::from_tiles_dir([a?, c?, d?], b?, dir_p),
                3 => Daiminkan::from_tiles_dir([b?, c?, d?], a?, dir_p),
                _ => panic!(),
            }.map(|daiminkan| Meld::Daiminkan(daiminkan))
        }
        "a" => {
            Ankan::from_tiles([a?, b?, c?, d?]).map(|ankan| Meld::Ankan(ankan))
        }
        _ => None
    }
}

pub fn to_tenhou_meld(meld: &Meld) -> String {
    match meld {
        Meld::Chii(chii) => {
            let o0 = to_tenhou_tile(chii.own[0]);
            let o1 = to_tenhou_tile(chii.own[1]);
            let c = to_tenhou_tile(chii.called);
            format!("c{}{}{}", c, o0, o1)
        }
        Meld::Pon(pon) => {
            let o0 = to_tenhou_tile(pon.own[0]);
            let o1 = to_tenhou_tile(pon.own[1]);
            let c = to_tenhou_tile(pon.called);
            match pon.dir.to_u8() {
                1 => format!("{}{}p{}", o0, o1, c),
                2 => format!("{}p{}{}", o0, c, o1),
                3 => format!("p{}{}{}", c, o0, o1),
                _ => panic!()
            }
        }
        Meld::Kakan(kakan) => {
            let o0 = to_tenhou_tile(kakan.pon.own[0]);
            let o1 = to_tenhou_tile(kakan.pon.own[1]);
            let c = to_tenhou_tile(kakan.pon.called);
            let a = to_tenhou_tile(kakan.added);
            match kakan.pon.dir.to_u8() {
                1 => format!("{}{}k{}{}", o0, o1, a, c),
                2 => format!("{}k{}{}{}", o0, a, c, o1),
                3 => format!("k{}{}{}{}", a, c, o0, o1),
                _ => panic!()
            }
        }
        Meld::Daiminkan(daiminkan) => {
            let o0 = to_tenhou_tile(daiminkan.own[0]);
            let o1 = to_tenhou_tile(daiminkan.own[1]);
            let o2 = to_tenhou_tile(daiminkan.own[2]);
            let c = to_tenhou_tile(daiminkan.called);
            match daiminkan.dir.to_u8() {
                1 => format!("{}{}{}m{}", o0, o1, o2, c),
                2 => format!("{}m{}{}{}", o0, c, o1, o2),
                3 => format!("m{}{}{}{}", c, o0, o1, o2),
                _ => panic!()
            }
        }
        Meld::Ankan(ankan) => {
            let o = ankan.own.map(to_tenhou_tile);
            format!("{}{}{}a{}", o[0], o[1], o[2], o[3])
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use pretty_assertions::assert_eq;
    use super::*;

    fn t(s: &str) -> Tile { Tile::from_str(s).unwrap() }

    #[test]
    fn tenhou_melds() {
        let str_meld = [
            ("c181617", Meld::Chii(Chii::from_tiles(
                t("6m"), t("7m"), t("8m")
            ).unwrap())),
            ("c262527", Meld::Chii(Chii::from_tiles(
                t("5p"), t("7p"), t("6p")
            ).unwrap())),
            ("c313233", Meld::Chii(Chii::from_tiles(
                t("2s"), t("3s"), t("1s")
            ).unwrap())),
            ("c533436", Meld::Chii(Chii::from_tiles(
                t("4s"), t("6s"), t("0s")
            ).unwrap())),
            ("c345336", Meld::Chii(Chii::from_tiles(
                t("0s"), t("6s"), t("4s")
            ).unwrap())),

            ("1212p12", Meld::Pon(Pon::from_tiles_dir(
                t("2m"), t("2m"), t("2m"),Player::new(1)
            ).unwrap())),
            ("12p1212", Meld::Pon(Pon::from_tiles_dir(
                t("2m"), t("2m"), t("2m"),Player::new(2)
            ).unwrap())),
            ("p121212", Meld::Pon(Pon::from_tiles_dir(
                t("2m"), t("2m"), t("2m"),Player::new(3)
            ).unwrap())),
            ("25p5225", Meld::Pon(Pon::from_tiles_dir(
                t("5p"), t("5p"), t("0p"),Player::new(2)
            ).unwrap())),

            ("242424a24", Meld::Ankan(Ankan::from_tiles(
                [t("4p"),t("4p"),t("4p"),t("4p")]
            ).unwrap())),
        ];
        for (s, m) in str_meld.into_iter() {
            let parsed = parse_tenhou_meld(s).unwrap();
            let serialized = to_tenhou_meld(&m);
            assert_eq!(parsed, m);
            assert_eq!(serialized, s);
        }
    }
}
