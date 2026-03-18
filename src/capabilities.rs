use serde::Deserialize;
use serde::Serialize;
use serde_with::DefaultOnError;
use serde_with::serde_as;
use std::collections::HashMap;

#[derive(PartialEq, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Capabilities {
    pub grants: Vec<Grant>,
    pub labels: HashMap<String, String>,
}

#[derive(PartialEq, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Grant {
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

    #[serde(
        serialize_with = "crate::serde_utils::serialize_patterns",
        deserialize_with = "crate::serde_utils::deserialize_patterns"
    )]
    pub user_agents: Vec<glob::Pattern>,
}

impl Capabilities {
    pub fn deny_all() -> Self {
        Self {
            grants: vec![],
            labels: HashMap::new(),
        }
    }
}

impl From<Capabilities> for crate::authorization::Authorization {
    fn from(caps: Capabilities) -> Self {
        Self {
            rules: caps
                .grants
                .into_iter()
                .map(|c| crate::authorization::Rule {
                    providers: c.providers,
                    models: c.models,
                    user_agents: c.user_agents,
                })
                .collect(),
        }
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
    #[serde(default, rename = "flare.io/cap/lacuna/grants")]
    grants: Vec<Option<Grant>>,

    #[serde_as(as = "Vec<DefaultOnError<Option<_>>>")]
    #[serde(default, rename = "flare.io/cap/lacuna/labels")]
    labels: Vec<Option<HashMap<String, String>>>,
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

    let capabilities = Capabilities {
        grants: ts_capabilities.grants.into_iter().flatten().collect(),
        labels: {
            let mut map: HashMap<String, Vec<String>> = HashMap::new();
            // Collect labels from list of capabilities.
            for (k, v) in ts_capabilities
                .labels
                .into_iter()
                .flatten()
                .flat_map(|m| m.into_iter())
            {
                map.entry(k).or_default().push(v);
            }
            // Merge label with the same key with a ",".join
            map.into_iter()
                .map(|(k, mut vs)| {
                    vs.sort();
                    (k, vs.join(","))
                })
                .collect()
        },
    };

    Ok(capabilities)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(s: &str) -> glob::Pattern {
        glob::Pattern::new(s).unwrap()
    }

    #[test]
    fn test_grant_deserialize() {
        let json = r#"{"providers": ["myprovider", "prefix-*"], "models": ["claude-*"], "user_agents": ["python-*"]}"#;
        let grant: Grant = serde_json::from_str(json).unwrap();
        assert_eq!(
            grant,
            Grant {
                providers: vec![pattern("myprovider"), pattern("prefix-*")],
                models: vec![pattern("claude-*")],
                user_agents: vec![pattern("python-*")],
            },
        );
    }

    #[test]
    fn test_grant_deserialize_no_models() {
        let json = r#"{"providers": ["myprovider"]}"#;
        let grant: Grant = serde_json::from_str(json).unwrap();
        assert_eq!(
            grant,
            Grant {
                providers: vec![pattern("myprovider")],
                models: vec![],
                user_agents: vec![],
            },
        );
    }

    #[test]
    fn test_grant_deserialize_invalid_pattern() {
        let json = r#"{"providers": ["valid", "[invalid"]}"#;
        let result: Result<Grant, _> = serde_json::from_str(json);
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Pattern syntax error")
        );
    }

    #[test]
    fn test_grant_serialize() {
        let grant = Grant {
            providers: vec![pattern("myprovider"), pattern("prefix-*")],
            models: vec![pattern("claude-*")],
            user_agents: vec![pattern("python-*")],
        };
        let json = serde_json::to_value(&grant).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "providers": ["myprovider", "prefix-*"],
                "models": ["claude-*"],
                "user_agents": ["python-*"],
            }),
        );
    }

    #[test]
    fn parse_valid_capabilities() {
        let json = r#"{
            "flare.io/cap/lacuna/grants": [
                {"providers": ["firstprovider"], "models": ["claude-*"], "user_agents": ["python-*"]},
                {"providers": ["secondprofider", "thirdprovider"], "models": ["gpt-*"]}
            ],
            "flare.io/cap/lacuna/labels": [
                {"team": "platform", "env": "production"},
                {"team": "infra"}
            ]
        }"#;
        let capabilities = parse_capabilities(json).unwrap();
        assert_eq!(
            capabilities,
            Capabilities {
                grants: vec![
                    Grant {
                        providers: vec![pattern("firstprovider")],
                        models: vec![pattern("claude-*")],
                        user_agents: vec![pattern("python-*")],
                    },
                    Grant {
                        providers: vec![pattern("secondprofider"), pattern("thirdprovider")],
                        models: vec![pattern("gpt-*")],
                        user_agents: vec![],
                    },
                ],
                labels: HashMap::from([
                    ("team".to_owned(), "infra,platform".to_owned()),
                    ("env".to_owned(), "production".to_owned()),
                ]),
            }
        );
    }

    #[test]
    fn parse_capabilities_invalid_ignored() {
        let json = r#"{
            "flare.io/cap/lacuna/grants": [
                {"providers": ["firstprovider"], "models": ["claude-*"]},
                ["something-bad"],
                {"providers": ["secondprofider"]}
            ]
        }"#;
        let capabilities = parse_capabilities(json).unwrap();
        assert_eq!(
            capabilities,
            Capabilities {
                grants: vec![
                    Grant {
                        providers: vec![pattern("firstprovider")],
                        models: vec![pattern("claude-*")],
                        user_agents: vec![],
                    },
                    Grant {
                        providers: vec![pattern("secondprofider")],
                        models: vec![],
                        user_agents: vec![],
                    },
                ],
                labels: Default::default(),
            }
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
        // Q-encoded: {"flare.io/cap/lacuna/grants":[{"providers":["🐿️"]}]}
        let encoded = r#"=?utf-8?q?{"flare.io/cap/lacuna/grants":[{"providers":["=F0=9F=90=BF=EF=B8=8F"]}]}?="#;
        let capabilities = parse_capabilities(encoded).unwrap();
        assert_eq!(
            capabilities,
            Capabilities {
                grants: vec![Grant {
                    providers: vec![pattern("🐿️")],
                    models: vec![],
                    user_agents: vec![],
                },],
                labels: Default::default()
            },
        );
    }
}
