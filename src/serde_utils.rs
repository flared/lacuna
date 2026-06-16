use serde::{Deserialize, Serialize};

pub fn serialize_patterns<S: serde::Serializer>(
    patterns: &[glob::Pattern],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let strings: Vec<&str> = patterns.iter().map(|p| p.as_str()).collect();
    strings.serialize(serializer)
}

pub fn deserialize_patterns<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<glob::Pattern>, D::Error> {
    let strings: Vec<String> = Vec::deserialize(deserializer)?;
    strings
        .into_iter()
        .map(|s| glob::Pattern::new(&s).map_err(serde::de::Error::custom))
        .collect()
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
/// Preserves declaration order and omits the `rewrite` key when `None`.
pub fn serialize_model_rules<S: serde::Serializer>(
    rules: &[crate::config::ModelRule],
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

/// Deserialize the ordered object form (`pattern => { "rewrite"?: target }`)
/// into `Vec<ModelRule>` to preserve JSON key order.
pub fn deserialize_model_rules<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<crate::config::ModelRule>, D::Error> {
    struct ModelRulesVisitor;

    impl<'de> serde::de::Visitor<'de> for ModelRulesVisitor {
        type Value = Vec<crate::config::ModelRule>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a map of model pattern to rule settings")
        }

        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut rules = Vec::new();
            while let Some((key, value)) = map.next_entry::<String, ModelRuleRepr>()? {
                let pattern = glob::Pattern::new(&key).map_err(serde::de::Error::custom)?;
                rules.push(crate::config::ModelRule {
                    pattern,
                    rewrite: value.rewrite,
                });
            }
            Ok(rules)
        }
    }

    deserializer.deserialize_map(ModelRulesVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ModelRule;

    fn parse(json: &str) -> Result<Vec<ModelRule>, serde_json::Error> {
        let mut de = serde_json::Deserializer::from_str(json);
        deserialize_model_rules(&mut de)
    }

    fn dump(rules: &[ModelRule]) -> String {
        let mut buf = Vec::new();
        let mut ser = serde_json::Serializer::new(&mut buf);
        serialize_model_rules(rules, &mut ser).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn preserves_declaration_order_when_deserializing_model_rules_() {
        let rules = parse(r#"{ "c-*": {}, "a-*": {}, "b-*": {} }"#).unwrap();
        assert_eq!(
            rules,
            vec![
                ModelRule {
                    pattern: glob::Pattern::new("c-*").unwrap(),
                    rewrite: None,
                },
                ModelRule {
                    pattern: glob::Pattern::new("a-*").unwrap(),
                    rewrite: None,
                },
                ModelRule {
                    pattern: glob::Pattern::new("b-*").unwrap(),
                    rewrite: None,
                }
            ]
        );

        let patterns: Vec<&str> = rules.iter().map(|r| r.pattern.as_str()).collect();
        assert_eq!(patterns, vec!["c-*", "a-*", "b-*"]);
    }

    #[test]
    fn deserialization_rejected_when_model_rule_pattern_is_invalid() {
        let err = parse(r#"{ "[invalid": {} }"#).unwrap_err().to_string();
        assert!(
            err.contains("Pattern syntax error"),
            "expected pattern syntax error, got: {err}"
        );
    }

    #[test]
    fn deserialization_rejected_when_model_rule_has_unknown_field() {
        let err = parse(r#"{ "a-*": { "something": "x" } }"#)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("unknown field"),
            "expected unknown field error, got: {err}"
        );
    }

    #[test]
    fn serialization_deserialization_round_trip() {
        let rules = vec![
            ModelRule {
                pattern: glob::Pattern::new("a-*").unwrap(),
                rewrite: Some("X".to_string()),
            },
            ModelRule {
                pattern: glob::Pattern::new("b-*").unwrap(),
                rewrite: None,
            },
        ];
        assert_eq!(dump(&rules), r#"{"a-*":{"rewrite":"X"},"b-*":{}}"#);
        // And it round-trips back to the same rules.
        assert_eq!(parse(&dump(&rules)).unwrap(), rules);
    }
}
