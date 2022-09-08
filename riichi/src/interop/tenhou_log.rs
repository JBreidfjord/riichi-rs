mod end_info;
mod entry;
mod meld;
mod recovery;
mod round;
mod scoring;
pub mod test_utils;
pub mod strings;
mod tile;

use serde::{
    Serialize, Deserialize,
};

pub use self::{
    end_info::*,
    entry::*,
    meld::*,
    recovery::*,
    round::*,
    scoring::*,
    tile::*,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]  // No need to compare other than in tests.
pub struct TenhouLog {
    #[serde(rename = "log")]
    pub rounds: Vec<TenhouRoundRaw>,

    pub rule: TenhouRule,

    // misc metadata below

    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub player_names: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]  // No need to compare other than in tests.
pub struct TenhouRule {
    #[serde(rename = "disp")]
    pub raw_rule_str: String,

    #[serde(rename = "aka51", skip_serializing_if = "Option::is_none")]
    pub num_reds_0: Option<u8>,
    #[serde(rename = "aka52", skip_serializing_if = "Option::is_none")]
    pub num_reds_1: Option<u8>,
    #[serde(rename = "aka53", skip_serializing_if = "Option::is_none")]
    pub num_reds_2: Option<u8>,
    #[serde(rename = "aka", skip_serializing_if = "Option::is_none")]
    pub num_reds_each: Option<u8>,
}

impl TenhouRule {
    pub fn num_reds(&self) -> [u8; 3] {
        if let (Some(m), Some(p), Some(s)) = (self.num_reds_0, self.num_reds_1, self.num_reds_2) {
            [m, p, s]
        } else if let Some(a) = self.num_reds_each {
            [a, a, a]
        } else if self.raw_rule_str.find("赤").is_some() {
            [1, 1, 1]
        } else {
            [0, 0, 0]
        }
    }

    pub fn allows_kuitan(&self) -> bool {
        self.raw_rule_str.find("喰").is_some()
    }

    pub fn num_kyokus(&self) -> Option<u8> {
        if self.raw_rule_str.find("東").is_some() {
            Some(4)
        } else if self.raw_rule_str.find("南").is_some() {
            Some(8)
        } else {
            None
        }
    }
}
