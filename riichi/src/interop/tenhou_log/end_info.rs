use std::fmt::Formatter;
use serde::{Deserialize, Deserializer};
use serde::de::{SeqAccess, Visitor, Error};
use crate::common::*;
use crate::model::*;
use super::strings::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndInfo {
    pub result: ActionResult,
    pub agari: Vec<AgariResult>,
}

impl<'de> Deserialize<'de> for EndInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_seq(EndInfoVisitor)
    }
}

struct EndInfoVisitor;

impl<'de> Visitor<'de> for EndInfoVisitor {
    type Value = EndInfo;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter,
            "a tenhou6 result array with at least 1 string element describing the overall result\
            of the round"
        )
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        let result_str: String = seq.next_element()?.ok_or_else(|| Error::custom("no result str"))?;
        if result_str != AGARI_STR {
            // This has to be an abort result.
            return Ok(EndInfo {
                result: action_result_from_str(&result_str)
                    .ok_or_else(|| Error::custom("unrecognized result"))?,
                agari: vec![],
            })
        }
        let mut agari_results: Vec<AgariResult> = vec![];
        while let (
            Some(points),
            Some(details),
        ) =
        (
            seq.next_element::<serde_json::Value>()?,
            seq.next_element::<serde_json::Value>()?
        ) {
            dbg!(points);
            dbg!(details);
            // TODO(summivox): nested deserializers for both points and details.
            agari_results.push(AgariResult::default())
        }
        Ok(EndInfo {
            result: agari_results[0].kind.to_action_result(),
            agari: agari_results,
        })
        // TODO: details
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abort_example() {
        let end_info_struct = EndInfo {
            result: ActionResult::AbortFourRiichi,
            agari: vec![],
        };
        let end_info_json = serde_json::json!(["四家立直"]);
        let deserialized: EndInfo = serde_json::from_value(end_info_json).unwrap();
        assert_eq!(end_info_struct, deserialized);
    }

    #[test]
    fn ron_2_example() {
        let end_info_struct = EndInfo {
            result: ActionResult::RonAgari,
            agari: vec![AgariResult::default(), AgariResult::default()],
        };
        let end_info_json = serde_json::json!([
            "和了",
            [13000, 0, 0, -12000],
            [0, 3, 0, "跳満12000点", "立直(1飜)", "ドラ(3飜)", "赤ドラ(2飜)"],
            [0, 0, 2000, -2000],
            [2, 3, 2, "30符2飜2000点", "役牌 發(1飜)", "ドラ(1飜)"]
        ]);
        let deserialized: EndInfo = serde_json::from_value(end_info_json).unwrap();
        assert_eq!(end_info_struct, deserialized);
    }
}
