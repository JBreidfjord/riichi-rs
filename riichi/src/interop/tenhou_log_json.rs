//! Conversion of Tenhou's JSON format game logs (a.k.a. "tenhou/6") <=> our data model.
//!
//! Intermediate Serde data models are defined to directly interface with the JSON, rooting at
//! [`TenhouLog`], with [`TenhouRoundRaw`] being centric to the format (represents one round).
//!
//! Each [`TenhouRoundRaw`] can be processed to recover both its boundary conditions (start and end)
//! and each of its turn's action and reaction(s).
//! This is done by [`recovery::recover_round`] into [`recovery::RecoveredRound`].
//!
//! Semantics of the format is largely reverse engineered from publicly available JSON logs.
//! There are hidden / lesser known features, but the bulk has been implemented here.
//!
//! - Japanese only introduction: <https://tenhou.net/mjlog.html>
//! - Actual tool: <http://tenhou.net/5>, <http://tenhou.net/6>

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

/// Root Serde model; corresponds to the outermost JSON log object.
///
/// Many metadata fields are optional and not completely defined here.
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

/// Serde model for rules; recognizes the most common and important fields.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]  // No need to compare other than in tests.
pub struct TenhouRule {
    #[serde(rename = "disp")]
    pub raw_rule_str: String,

    #[serde(rename = "aka51")]
    pub num_reds_0: Option<u8>,
    #[serde(rename = "aka52")]
    pub num_reds_1: Option<u8>,
    #[serde(rename = "aka53")]
    pub num_reds_2: Option<u8>,
    #[serde(rename = "aka")]
    pub num_reds_each: Option<u8>,
}

impl TenhouRule {
    /// Returns the indicated number of red tiles in the (complete) wall.
    /// Note that `raw_rule_str` may override whether reds are considered or not.
    pub fn num_reds(&self) -> [u8; 3] {
        if let (Some(m), Some(p), Some(s)) = (self.num_reds_0, self.num_reds_1, self.num_reds_2) {
            [m, p, s]
        } else if let Some(a) = self.num_reds_each {
            [a, a, a]
        } else {
            [0, 0, 0]
        }
    }

    /// Returns `Some(true)` if reds are explicitly allowed, `Some(false)` if explicitly disallowed,
    /// or `None` if unspecified.
    pub fn allows_red(&self) -> Option<bool> {
        (!self.raw_rule_str.is_empty()).then(|| self.raw_rule_str.contains("喰"))
    }

    /// Returns whether the Tanyao yaku is allowed in an open hand.
    pub fn allows_kuitan(&self) -> bool {
        self.raw_rule_str.contains("喰")
    }

    /// Returns whether this is an East-only game (4 kyokus), or an East-South game (8 kyokus).
    /// `None` if unknown from the raw rule string.
    pub fn num_kyokus(&self) -> Option<u8> {
        if self.raw_rule_str.contains("東") {
            Some(4)
        } else if self.raw_rule_str.contains("南") {
            Some(8)
        } else {
            None
        }
    }
}
