use crate::matching::most_specific_match;
use crate::model_rules::ModelRule;

pub fn get_rewritten_name(model: &str, rules: &[ModelRule]) -> Option<String> {
    most_specific_match(rules, model, |r| &r.pattern).and_then(|r| r.rewrite.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_rule(pattern: &str, rewrite: Option<&str>) -> ModelRule {
        ModelRule {
            pattern: glob::Pattern::new(pattern).unwrap(),
            rewrite: rewrite.map(|r| r.to_owned()),
        }
    }

    #[test]
    fn rewrite_when_matching() {
        let rules = vec![
            model_rule("gemini-*", Some("something")),
            model_rule("claude-*", Some("target")),
        ];
        assert_eq!(
            get_rewritten_name("claude-opus", &rules),
            Some("target".to_owned())
        );
    }

    #[test]
    fn no_rewrite_when_rewrite_rule_is_none() {
        let rules = vec![model_rule("claude-*", None)];
        assert_eq!(get_rewritten_name("claude-opus", &rules), None);
    }
}
