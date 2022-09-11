//! Game rules and variations.

/// TODO(summivox): rules

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rules{}

impl Default for Rules {
    fn default() -> Self {
        // TODO(summivox): rules: default values should agree with Tenhou/Majsoul
        Self{}
    }
}
