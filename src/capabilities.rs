use serde::Deserialize;
use serde::Serialize;
use serde_with::DefaultOnError;
use serde_with::serde_as;

#[derive(PartialEq, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Capabilities {
    pub capabilities: Vec<Capability>,
}

#[derive(PartialEq, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Capability {
    #[serde(
        serialize_with = "crate::serde_utils::serialize_patterns",
        deserialize_with = "crate::serde_utils::deserialize_patterns"
    )]
    pub providers: Vec<glob::Pattern>,

    #[serde(
        serialize_with = "crate::serde_utils::serialize_patterns",
        deserialize_with = "crate::serde_utils::deserialize_patterns"
    )]
    pub models: Vec<glob::Pattern>,
}

#[derive(Debug)]
pub enum MatchedModel<'a> {
    Unknown,
    Some(&'a str),
    None,
}

impl Capabilities {
    pub fn from_capabilities(capabilities: Vec<Capability>) -> Self {
        Self { capabilities }
    }

    pub fn is_allowed(&self, provider: &str, model: &MatchedModel) -> bool {
        self.capabilities.iter().any(|c| {
            let provider_matches =
                c.providers.is_empty() || c.providers.iter().any(|p| p.matches(provider));
            let model_matches = c.models.is_empty()
                || match model {
                    MatchedModel::Unknown => {
                        // We failed to identify the model.
                        // The best we can do is check if any model is allowed.
                        c.models.iter().any(|p| p.matches(""))
                    }
                    MatchedModel::Some(m) => c.models.iter().any(|p| p.matches(m)),
                    MatchedModel::None => true,
                };
            provider_matches && model_matches
        })
    }

    pub fn deny_all() -> Self {
        Self::default()
    }
}

// Tailscale capabilities are a JSON object where keys are capability
// names and values are arrays of arbitrary JSON objects.
//
// Ref: https://tailscale.com/docs/features/access-control/grants/grants-app-capabilities
#[serde_as]
#[derive(Debug, Deserialize)]
struct TailscaleCapabilities {
    #[serde_as(as = "Vec<DefaultOnError<Option<_>>>")]
    #[serde(default, rename = "flare.io/cap/lacuna")]
    app_capabilities: Vec<Option<Capability>>,
}

pub fn parse_capabilities(header_value: &str) -> Result<Capabilities, anyhow::Error> {
    let decoded = rfc2047_decoder::Decoder::new()
        .too_long_encoded_word_strategy(rfc2047_decoder::RecoverStrategy::Decode)
        .decode(header_value.as_bytes())
        .unwrap_or_else(|_| header_value.to_owned());

    let ts_capabilities: TailscaleCapabilities = match serde_json::from_str(&decoded) {
        Ok(v) => v,
        Err(e) => {
            return Err(anyhow::anyhow!("failed to parse capabilities header: {e}"));
        }
    };

    let capabilities = Capabilities::from_capabilities(
        ts_capabilities
            .app_capabilities
            .into_iter()
            .flatten()
            .collect(),
    );

    Ok(capabilities)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(s: &str) -> glob::Pattern {
        glob::Pattern::new(s).unwrap()
    }

    #[test]
    fn test_is_allowed() {
        let capabilities = Capabilities::from_capabilities(vec![Capability {
            providers: vec![pattern("providerone"), pattern("providerprefix-*")],
            models: vec![pattern("claude-*"), pattern("gpt-4o")],
        }]);

        // Provider with MatchedModel::None (only checks provider)
        assert!(capabilities.is_allowed("providerone", &MatchedModel::None));
        assert!(capabilities.is_allowed("providerprefix-suffix", &MatchedModel::None));
        assert!(!capabilities.is_allowed("providertwo", &MatchedModel::None));

        // Provider with MatchedModel::Some (checks both provider and model)
        assert!(capabilities.is_allowed(
            "providerone",
            &MatchedModel::Some("claude-sonnet-4-20250514")
        ));
        assert!(capabilities.is_allowed("providerone", &MatchedModel::Some("gpt-4o")));
        assert!(!capabilities.is_allowed("providerone", &MatchedModel::Some("gpt-3.5-turbo")));

        // Wrong provider
        assert!(!capabilities.is_allowed("other", &MatchedModel::Some("claude-sonnet-4-20250514")));
    }

    #[test]
    fn test_is_allowed_unknown_model() {
        // Unknown model is authorized if the capability allows any model (wildcard).
        let with_wildcard = Capabilities::from_capabilities(vec![Capability {
            providers: vec![pattern("p")],
            models: vec![pattern("**")],
        }]);
        assert!(with_wildcard.is_allowed("p", &MatchedModel::Unknown));

        // Unknown model is authorized if the capability allows any model (empty list)
        let empty_models = Capabilities::from_capabilities(vec![Capability {
            providers: vec![pattern("p")],
            models: vec![],
        }]);
        assert!(empty_models.is_allowed("p", &MatchedModel::Unknown));

        // Unknown model is blocked if the capability requires a specific model.
        let without_wildcard = Capabilities::from_capabilities(vec![Capability {
            providers: vec![pattern("p")],
            models: vec![pattern("claude-*")],
        }]);
        assert!(!without_wildcard.is_allowed("p", &MatchedModel::Unknown));
    }

    #[test]
    fn test_is_allowed_empty_list_allows_any() {
        // Empty models list means "all models allowed"
        let empty_models = Capabilities::from_capabilities(vec![Capability {
            providers: vec![pattern("p")],
            models: vec![],
        }]);
        assert!(empty_models.is_allowed("p", &MatchedModel::Some("anything")));

        // Empty providers list means "all providers allowed"
        let empty_providers = Capabilities::from_capabilities(vec![Capability {
            providers: vec![],
            models: vec![pattern("claude-*")],
        }]);
        assert!(empty_providers.is_allowed(
            "anyprovider",
            &MatchedModel::Some("claude-sonnet-4-20250514")
        ));
    }

    #[test]
    fn test_is_allowed_multiple_capabilities() {
        let capabilities = Capabilities::from_capabilities(vec![
            Capability {
                providers: vec![pattern("provider-a")],
                models: vec![pattern("claude-*")],
            },
            Capability {
                providers: vec![pattern("provider-b")],
                models: vec![pattern("gpt-*")],
            },
        ]);
        assert!(capabilities.is_allowed(
            "provider-a",
            &MatchedModel::Some("claude-sonnet-4-20250514")
        ));
        assert!(!capabilities.is_allowed("provider-a", &MatchedModel::Some("gpt-4o")));
        assert!(capabilities.is_allowed("provider-b", &MatchedModel::Some("gpt-4o")));
        assert!(!capabilities.is_allowed(
            "provider-b",
            &MatchedModel::Some("claude-sonnet-4-20250514")
        ));
    }

    #[test]
    fn test_capability_deserialize() {
        let json = r#"{"providers": ["myprovider", "prefix-*"], "models": ["claude-*"]}"#;
        let capability: Capability = serde_json::from_str(json).unwrap();
        assert_eq!(
            capability,
            Capability {
                providers: vec![pattern("myprovider"), pattern("prefix-*")],
                models: vec![pattern("claude-*")],
            },
        );
    }

    #[test]
    fn test_capability_deserialize_no_models() {
        let json = r#"{"providers": ["myprovider"]}"#;
        let capability: Capability = serde_json::from_str(json).unwrap();
        assert_eq!(
            capability,
            Capability {
                providers: vec![pattern("myprovider")],
                models: vec![],
            },
        );
    }

    #[test]
    fn test_capability_deserialize_invalid_pattern() {
        let json = r#"{"providers": ["valid", "[invalid"]}"#;
        let result: Result<Capability, _> = serde_json::from_str(json);
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Pattern syntax error")
        );
    }

    #[test]
    fn test_capability_serialize() {
        let capability = Capability {
            providers: vec![pattern("myprovider"), pattern("prefix-*")],
            models: vec![pattern("claude-*")],
        };
        let json = serde_json::to_value(&capability).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "providers": ["myprovider", "prefix-*"],
                "models": ["claude-*"],
            }),
        );
    }

    #[test]
    fn parse_valid_capabilities() {
        let json = r#"{
            "flare.io/cap/lacuna": [
                {"providers": ["firstprovider"], "models": ["claude-*"]},
                {"providers": ["secondprofider", "thirdprovider"], "models": ["gpt-*"]}
            ]
        }"#;
        let capabilities = parse_capabilities(json).unwrap();
        assert_eq!(
            capabilities,
            Capabilities::from_capabilities(vec![
                Capability {
                    providers: vec![pattern("firstprovider")],
                    models: vec![pattern("claude-*")],
                },
                Capability {
                    providers: vec![pattern("secondprofider"), pattern("thirdprovider")],
                    models: vec![pattern("gpt-*")],
                },
            ]),
        );
    }

    #[test]
    fn parse_capabilities_invalid_ignored() {
        let json = r#"{
            "flare.io/cap/lacuna": [
                {"providers": ["firstprovider"], "models": ["claude-*"]},
                ["something-bad"],
                {"providers": ["secondprofider"]}
            ]
        }"#;
        let capabilities = parse_capabilities(json).unwrap();
        assert_eq!(
            capabilities,
            Capabilities::from_capabilities(vec![
                Capability {
                    providers: vec![pattern("firstprovider")],
                    models: vec![pattern("claude-*")],
                },
                Capability {
                    providers: vec![pattern("secondprofider")],
                    models: vec![],
                },
            ]),
        );
    }

    #[test]
    fn parse_missing_key_returns_none() {
        let json = r#"{"other.key": []}"#;
        let capabilities = parse_capabilities(json).unwrap();
        assert_eq!(capabilities, Capabilities::default());
    }

    #[test]
    fn parse_malformed_json_returns_err() {
        assert!(parse_capabilities("not json").is_err());
    }

    #[test]
    fn parse_rfc2047_encoded() {
        // Tailscale uses RFC2047 "Q" encoding for values that contain non-ASCII characters.
        // Ref: https://tailscale.com/docs/features/tailscale-serve#app-capabilities-header
        // Q-encoded: {"flare.io/cap/lacuna":[{"providers":["🐿️"]}]}
        let encoded =
            r#"=?utf-8?q?{"flare.io/cap/lacuna":[{"providers":["=F0=9F=90=BF=EF=B8=8F"]}]}?="#;
        let capabilities = parse_capabilities(encoded).unwrap();
        assert_eq!(
            capabilities,
            Capabilities::from_capabilities(vec![Capability {
                providers: vec![pattern("🐿️")],
                models: vec![],
            },]),
        );
    }
}
