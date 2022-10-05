use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

use crate::rules::Ruleset;
use super::{Yaku, STANDARD_YAKU, get_blocked_yaku};

pub type YakuValues = HashMap<Yaku, i8>;

#[derive(Debug, Default)]
pub struct YakuBuilder {
    allowed_extra_yaku: HashSet<Yaku>,
    yaku_values: YakuValues,
    blocked_yaku: HashSet<Yaku>,
    has_yakuman: bool,
}

impl YakuBuilder {
    pub fn new(ruleset: &Ruleset) -> Self {
        Self {
            allowed_extra_yaku: ruleset.yaku_extra.clone(),
            yaku_values: Default::default(),
            blocked_yaku: ruleset.yaku_block.clone(),
            has_yakuman: false,
        }
    }

    pub fn add(&mut self, yaku: Yaku, value: i8) {
        if self.has_yakuman && value > 0 { return }
        if self.blocked_yaku.contains(&yaku) { return }
        if !(STANDARD_YAKU.contains(&yaku) ||
            self.allowed_extra_yaku.contains(&yaku)) { return }

        self.yaku_values.insert(yaku, value);
        for blocked in get_blocked_yaku(yaku) {
            self.yaku_values.remove(blocked);
            self.blocked_yaku.insert(*blocked);
        }
        if !self.has_yakuman && value < 0 {
            self.has_yakuman = true;
            self.yaku_values.retain(|_, other_value| *other_value < 0);
        }
    }

    pub fn build(self) -> YakuValues { self.yaku_values }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaku_builder() {
        use Yaku::*;

        let mut x = YakuBuilder::new(&Ruleset::default());
        x.add(Chinroutou, -1);
        x.add(Honchantaiyaochuu, 1);
        let a = x.build();
        assert_eq!(a, YakuValues::from_iter([(Chinroutou, -1)]));

        let mut x = YakuBuilder::new(&Ruleset::default());
        x.add(Honchantaiyaochuu, 1);
        x.add(Chinroutou, -1);
        let a = x.build();
        assert_eq!(a, YakuValues::from_iter([(Chinroutou, -1)]));
    }
}
