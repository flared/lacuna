use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct ModelRule {
    pub pattern: glob::Pattern,
    pub rewrite: Option<String>,
}

impl ModelRule {
    /// True when the rule has attributes set beyond its pattern.
    /// Exhaustive destructure is deliberate: adding a field forces this to be revisited.
    pub(crate) fn has_settings(&self) -> bool {
        let Self {
            pattern: _,
            rewrite,
        } = self;
        rewrite.is_some()
    }
}

/// Exists only so the map value can be (de)serialized with `deny_unknown_fields`
/// (rejecting typos such as `{ "rewite": "x" }`)
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelRuleRepr {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rewrite: Option<String>,
}

/// Serialize `Vec<ModelRule>` as an ordered object: `pattern => { "rewrite"?: target }`.
pub(crate) fn serialize_model_rules<S: serde::Serializer>(
    rules: &[ModelRule],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(rules.len()))?;
    for rule in rules {
        map.serialize_entry(
            rule.pattern.as_str(),
            &ModelRuleRepr {
                rewrite: rule.rewrite.clone(),
            },
        )?;
    }
    map.end()
}

/// Deserialize the map (`pattern => { "rewrite"?: target }`) into rules sorted
/// by pattern, so iteration order is deterministic regardless of the JSON key order.
pub(crate) fn deserialize_model_rules<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ModelRule>, D::Error> {
    let map = std::collections::BTreeMap::<String, ModelRuleRepr>::deserialize(deserializer)?;
    map.into_iter()
        .map(|(key, value)| {
            let pattern = glob::Pattern::new(&key).map_err(serde::de::Error::custom)?;
            Ok(ModelRule {
                pattern,
                rewrite: value.rewrite,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_model_rules(json: &str) -> Result<Vec<ModelRule>, serde_json::Error> {
        let mut de = serde_json::Deserializer::from_str(json);
        deserialize_model_rules(&mut de)
    }

    #[test]
    fn deserializes_model_rules_to_deterministic_sorted_order_regardless_of_input_order() {
        let a = parse_model_rules(r#"{ "c-*": {}, "a-*": {}, "b-*": {} }"#).unwrap();
        let b = parse_model_rules(r#"{ "b-*": {}, "c-*": {}, "a-*": {} }"#).unwrap();
        assert_eq!(a, b);
        let patterns: Vec<&str> = a.iter().map(|r| r.pattern.as_str()).collect();
        assert_eq!(patterns, vec!["a-*", "b-*", "c-*"]);
    }

    #[test]
    fn duplicate_model_rule_patterns_are_deduped_last_wins() {
        let rules = parse_model_rules(r#"{ "a-*": { "rewrite": "X" }, "a-*": {} }"#).unwrap();
        assert_eq!(
            rules,
            vec![ModelRule {
                pattern: glob::Pattern::new("a-*").unwrap(),
                rewrite: None,
            }]
        );
    }

    #[test]
    fn deserialization_rejected_when_model_rule_pattern_is_invalid() {
        let err = parse_model_rules(r#"{ "[invalid": {} }"#)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Pattern syntax error"),
            "expected pattern syntax error, got: {err}"
        );
    }

    #[test]
    fn deserialization_rejected_when_model_rule_has_unknown_field() {
        let err = parse_model_rules(r#"{ "a-*": { "something": "x" } }"#)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("unknown field"),
            "expected unknown field error, got: {err}"
        );
    }
}
