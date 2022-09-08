
use once_cell::sync::Lazy;
use regex::Regex;

use crate::common::*;
use super::tile::*;

pub fn parse_tenhou_meld(s: &str) -> Option<Meld> {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?x)
      ([cpmk])?(\d\d)  # 1 2 chii/pon/daiminkan/kakan
       ([pmk])?(\d\d)? # 3 4 pon/daiminkan/kakan
        ([pk])?(\d\d)? # 5 6 pon/kakan
        ([ma])?(\d\d)? # 7 8 daiminkan/ankan
    ").unwrap());
    let groups = RE.captures(s)?;
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
            // (a)bc
            Chii::from_tiles(b?, c?, a?)
                .map(Meld::Chii)
        }
        "p" => {
            let dir_p = Player::new(dir);
            match dir {
                // ab(c)
                1 => Pon::from_tiles_dir(a?, b?, c?, dir_p),
                // a(b)c
                2 => Pon::from_tiles_dir(a?, c?, b?, dir_p),
                // (a)bc
                3 => Pon::from_tiles_dir(b?, c?, a?, dir_p),
                _ => panic!(),
            }.map(Meld::Pon)
        }
        "k" => {
            let dir_p = Player::new(dir);
            match dir {
                // ab(c/d)
                1 => Pon::from_tiles_dir(a?, b?, d?, dir_p)
                    .and_then(|pon| Kakan::from_pon_added(pon, c?)),
                // a(b/c)d
                2 => Pon::from_tiles_dir(a?, d?, c?, dir_p)
                    .and_then(|pon| Kakan::from_pon_added(pon, b?)),
                // (a/b)cd
                3 => Pon::from_tiles_dir(c?, d?, b?, dir_p)
                    .and_then(|pon| Kakan::from_pon_added(pon, a?)),
                _ => panic!(),
            }.map(Meld::Kakan)
        }
        "m" => {
            let dir_p = Player::new(dir);
            match dir {
                // abc(d)
                1 => Daiminkan::from_tiles_dir([a?, b?, c?], d?, dir_p),
                // a(b)cd
                2 => Daiminkan::from_tiles_dir([a?, c?, d?], b?, dir_p),
                // (a)bcd
                3 => Daiminkan::from_tiles_dir([b?, c?, d?], a?, dir_p),
                _ => panic!(),
            }.map(Meld::Daiminkan)
        }
        "a" => {
            Ankan::from_tiles([a?, b?, c?, d?]).map(Meld::Ankan)
        }
        _ => None
    }
}

pub fn to_tenhou_meld(meld: &Meld) -> String {
    // NOTE: Resorting own tiles is necessary for melds other than Chii due to Tenhou's convention
    // on red tiles going last.
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
            let (o0, o1) = sort2(o0, o1);
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
            let (o0, o1) = sort2(o0, o1);
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
            let (o0, o1, o2) = sort3(o0, o1, o2);
            match daiminkan.dir.to_u8() {
                1 => format!("{}{}{}m{}", o0, o1, o2, c),
                2 => format!("{}m{}{}{}", o0, c, o1, o2),
                3 => format!("m{}{}{}{}", c, o0, o1, o2),
                _ => panic!()
            }
        }
        Meld::Ankan(ankan) => {
            let mut o = ankan.own.map(to_tenhou_tile);
            o.sort();
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
                t("2m"), t("2m"), t("2m"), P1
            ).unwrap())),
            ("12p1212", Meld::Pon(Pon::from_tiles_dir(
                t("2m"), t("2m"), t("2m"), P2
            ).unwrap())),
            ("p121212", Meld::Pon(Pon::from_tiles_dir(
                t("2m"), t("2m"), t("2m"), P3
            ).unwrap())),
            ("p151551", Meld::Pon(Pon::from_tiles_dir(
                t("0m"), t("5m"), t("5m"), P3
            ).unwrap())),
            ("p511515", Meld::Pon(Pon::from_tiles_dir(
                t("5m"), t("5m"), t("0m"), P3
            ).unwrap())),
            ("25p5225", Meld::Pon(Pon::from_tiles_dir(
                t("5p"), t("5p"), t("0p"), P2
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
