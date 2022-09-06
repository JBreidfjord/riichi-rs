mod end_info;
mod entry;
mod meld;
mod recovery;
mod round;
mod scoring;
mod strings;
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
