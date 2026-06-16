mod rewrite;
mod rule;

pub use rewrite::get_rewritten_name;
pub use rule::ModelRule;
pub(crate) use rule::{deserialize_model_rules, serialize_model_rules};

use crate::capabilities::Capabilities;
use crate::provider::Provider;

/// Provider rules merged with grant rewrites
pub fn merge_model_rules(
    provider: &Provider,
    caps: Option<&Capabilities>,
    user_agent: Option<&str>,
) -> Vec<ModelRule> {
    let grant_rules = caps
        .map(|c| c.collect_model_rules(&provider.key, user_agent))
        .unwrap_or_default();

    let mut merged: std::collections::BTreeMap<&str, ModelRule> = provider
        .model_rules
        .iter()
        .map(|r| (r.pattern.as_str(), r.clone()))
        .collect();

    for rule in &grant_rules {
        if rule.has_settings() {
            merged.insert(rule.pattern.as_str(), rule.clone());
        }
    }

    merged.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::Grant;
    use crate::provider::compatibility::Compatibility;

    fn model_rule(pattern: &str, rewrite: Option<&str>) -> ModelRule {
        ModelRule {
            pattern: glob::Pattern::new(pattern).unwrap(),
            rewrite: rewrite.map(|r| r.to_owned()),
        }
    }

    fn provider_with_rules(rules: Vec<ModelRule>) -> Provider {
        crate::test_utils::make_provider_with_model_rules(
            "p",
            "http://example.com",
            Compatibility::default(),
            rules,
        )
    }

    fn caps_with_rules(rules: Vec<ModelRule>) -> Capabilities {
        Capabilities {
            grants: vec![Grant {
                providers: vec![],
                model_rules: rules,
                user_agents: vec![],
            }],
            labels: Default::default(),
        }
    }

    #[test]
    fn grant_rewrite_overrides_provider_on_identical_pattern() {
        let provider = provider_with_rules(vec![model_rule("claude-*", Some("provider-target"))]);
        let caps = caps_with_rules(vec![model_rule("claude-*", Some("grant-target"))]);
        let merged = merge_model_rules(&provider, Some(&caps), None);

        assert_eq!(merged, vec![model_rule("claude-*", Some("grant-target"))]);
    }

    #[test]
    fn empty_grant_rule_does_not_remove_provider_rewrite() {
        let provider = provider_with_rules(vec![model_rule("claude-*", Some("provider-target"))]);
        let caps = caps_with_rules(vec![model_rule("claude-*", None)]);
        let merged = merge_model_rules(&provider, Some(&caps), None);

        assert_eq!(
            merged,
            vec![model_rule("claude-*", Some("provider-target"))]
        );
    }
}
