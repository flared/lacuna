pub use crate::provider::compatibility::Compatibility;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub lacuna: Lacuna,

    pub providers: HashMap<String, Provider>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Lacuna {
    #[serde(default)]
    pub logging: crate::logging::Logging,

    #[serde(default)]
    pub identity_header: Option<String>,

    #[serde(default)]
    pub capabilities_header: Option<String>,

    #[serde(default)]
    pub user_agents: Vec<crate::user_agent::UserAgentPatternConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelRule {
    pub pattern: glob::Pattern,
    pub rewrite: Option<String>,
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    #[serde(
        rename = "models",
        default,
        serialize_with = "serialize_model_rules",
        deserialize_with = "deserialize_model_rules"
    )]
    pub model_rules: Vec<ModelRule>,

    #[serde(
        default,
        serialize_with = "crate::serde_utils::serialize_patterns",
        deserialize_with = "crate::serde_utils::deserialize_patterns"
    )]
    pub user_agents: Vec<glob::Pattern>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub description: String,

    pub baseurl: String,

    #[serde(default)]
    pub capability: Capability,

    #[serde(default)]
    pub apikey: String,

    #[serde(default)]
    pub authorization: Authorization,

    #[serde(default)]
    pub tailnet: bool,

    #[serde(default)]
    pub compatibility: Compatibility,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Authorization {
    #[default]
    None,
    Bearer,
    XApiKey,
    XGoogApiKey,
}

impl FromStr for Config {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config: Config = json5::from_str(s)?;
        Ok(config)
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, anyhow::Error> {
        let contents = std::fs::read_to_string(path)?;
        let contents = Self::substitute_env_vars(&contents)?;
        contents.parse()
    }

    fn substitute_env_vars(input: &str) -> Result<String, anyhow::Error> {
        let re = regex::Regex::new(r"\$\{([^}]+)\}")?;
        let mut result = input.to_string();
        for caps in re.captures_iter(input) {
            let var_name = &caps[1];
            let value = std::env::var(var_name)
                .map_err(|_| anyhow::anyhow!("environment variable '{}' is not set", var_name))?;
            result = result.replace(&caps[0].to_string(), &value);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_model_rules(json: &str) -> Result<Vec<ModelRule>, serde_json::Error> {
        let mut de = serde_json::Deserializer::from_str(json);
        deserialize_model_rules(&mut de)
    }

    fn dump_model_rules(rules: &[ModelRule]) -> String {
        let mut buf = Vec::new();
        let mut ser = serde_json::Serializer::new(&mut buf);
        serialize_model_rules(rules, &mut ser).unwrap();
        String::from_utf8(buf).unwrap()
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
        assert_eq!(
            dump_model_rules(&rules),
            r#"{"a-*":{"rewrite":"X"},"b-*":{}}"#
        );
        // And it round-trips back to the same rules.
        assert_eq!(parse_model_rules(&dump_model_rules(&rules)).unwrap(), rules);
    }

    #[test]
    fn deserialize_json_config() {
        let json = r#"{
          "providers": {
            "openai": {
              "name": "OpenAI",
              "baseurl": "https://api.openai.com/v1",
              "capability": {
                "models": {
                  "gpt-4o": {},
                  "gpt-4o-mini": {}
                },
                "user_agents": ["claude-code"]
              },
              "apikey": "sk-test",
              "authorization": "bearer",
              "compatibility": {
                "openai_chat": true,
                "openai_responses": true
              }
            },
            "anthropic": {
              "name": "Anthropic",
              "baseurl": "https://api.anthropic.com",
              "capability": {
                "models": {
                  "claude-sonnet-4-20250514": {}
                }
              },
              "apikey": "sk-ant-test",
              "authorization": "x-api-key",
              "compatibility": {
                "anthropic_messages": true,
                "openai_chat": false
              }
            },
            "gemini": {
              "name": "Gemini",
              "baseurl": "https://generativelanguage.googleapis.com",
              "capability": {
                "models": {
                  "gemini-2.0-flash": {}
                }
              },
              "authorization": "x-goog-api-key",
              "headers": {
                "x-some-header": "foo"
              }
            }
          }
        }"#;
        let config: Config = json5::from_str(json).unwrap();
        assert_eq!(config.providers.len(), 3);

        let openai = &config.providers["openai"];
        assert_eq!(openai.name, "OpenAI");
        assert_eq!(openai.baseurl, "https://api.openai.com/v1");
        assert_eq!(
            openai.capability.model_rules,
            vec![
                ModelRule {
                    pattern: glob::Pattern::new("gpt-4o").unwrap(),
                    rewrite: None,
                },
                ModelRule {
                    pattern: glob::Pattern::new("gpt-4o-mini").unwrap(),
                    rewrite: None,
                },
            ]
        );
        assert_eq!(
            openai.capability.user_agents,
            vec![glob::Pattern::new("claude-code").unwrap()]
        );
        assert_eq!(openai.apikey, "sk-test");
        assert_eq!(openai.authorization, Authorization::Bearer);
        assert!(openai.compatibility.openai_chat);
        assert!(openai.compatibility.openai_responses);
        assert!(!openai.compatibility.anthropic_messages);

        let anthropic = &config.providers["anthropic"];
        assert_eq!(anthropic.authorization, Authorization::XApiKey);
        assert!(anthropic.compatibility.anthropic_messages);
        assert!(!anthropic.compatibility.openai_chat);

        let gemini = &config.providers["gemini"];
        assert_eq!(gemini.authorization, Authorization::XGoogApiKey);
        assert!(!gemini.compatibility.openai_chat);
        assert_eq!(
            gemini.headers,
            HashMap::from([("x-some-header".to_string(), "foo".to_string())])
        );
        assert!(!gemini.compatibility.openai_responses);
    }

    #[test]
    fn defaults_applied() {
        let json = r#"{
          "providers": {
            "minimal": {
              "name": "Minimal",
              "baseurl": "https://example.com",
              "capability": {
                "models": {
                  "model-1": {}
                }
              }
            }
          }
        }"#;
        let config: Config = json5::from_str(json).unwrap();
        let p = &config.providers["minimal"];
        assert_eq!(p.name, "Minimal");
        assert_eq!(p.apikey, "");
        assert_eq!(p.authorization, Authorization::None);
        assert!(p.headers.is_empty());
        assert!(!p.tailnet);
        assert_eq!(p.description, "");
        assert!(!p.compatibility.openai_chat);
        assert!(!p.compatibility.openai_responses);
        assert!(!p.compatibility.anthropic_messages);
    }

    #[test]
    fn load_with_env_substitution() {
        temp_env::with_var("LACUNA_TEST_API_KEY", Some("sk-from-env"), || {
            let dir = std::env::temp_dir().join("lacuna_test_env");
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("config_env.json5");
            std::fs::write(
                &path,
                r#"{
                  "providers": {
                    "openai": {
                      "baseurl": "https://api.openai.com/v1",
                      "apikey": "${LACUNA_TEST_API_KEY}"
                    }
                  }
                }"#,
            )
            .unwrap();
            let config = Config::load(&path).unwrap();
            assert_eq!(config.providers["openai"].apikey, "sk-from-env");
            std::fs::remove_dir_all(&dir).unwrap();
        });
    }

    #[test]
    fn deny_unknown_fields_in_lacuna() {
        // The "lacuna" object has no reason to contain unknown fields.
        // It should always be a mistake.
        let json = r#"{
          "lacuna": {
            "bad-key": "bad-value",
          },
        }"#;
        let err = Config::from_str(json).unwrap_err().to_string();
        assert!(
            err.contains("unknown field"),
            "expected error to mention 'unknown field', got: {err}"
        );
    }

    #[test]
    fn deserialize_doc_config() {
        // Deserialize config seen in doc: https://tailscale.com/docs/features/aperture/configuration#providers
        let json = r#"
        {
          "providers": {
            "openai": {
              "baseurl": "https://api.openai.com/",
              "apikey": "YOUR_OPENAI_KEY",
              "capability": {
                "models": {
                  "gpt-5": {},
                  "gpt-5-mini": {},
                  "gpt-4.1": {}
                },
                "user_agents": ["claude-code"]
              },
              "name": "OpenAI",
              "description": "OpenAI models",
              "compatibility": {
                "openai_chat": true,
                "openai_responses": true
              },
            },
            "bedrock": {
              "baseurl": "https://bedrock-runtime.us-east-1.amazonaws.com",
              "apikey": "bedrock-api-key-xxx",
              "authorization": "bearer",
              "capability": {
                "models": {
                  "us.anthropic.claude-haiku-4-5-20251001-v1:0": {},
                  "us.anthropic.claude-sonnet-4-5-20250929-v1:0": {},
                  "us.anthropic.claude-opus-4-5*": { "rewrite": "arn:aws:bedrock:us-east-1:409905535292:application-inference-profile/11cprf2uimr9" },
                  "us.anthropic.claude-opus-4-6-v1": {}
                }
              },
              "compatibility": {
                "bedrock_model_invoke": true
              }
            },
            "anthropic": {
              "baseurl": "https://api.anthropic.com",
              "apikey": "YOUR_ANTHROPIC_KEY",
              "authorization": "x-api-key",
              "capability": {
                "models": {
                  "claude-sonnet-4-5": {},
                  "claude-haiku-4-5": {},
                  "claude-opus-4-5": {}
                }
              },
              "compatibility": {
                "openai_chat": false,
                "anthropic_messages": true
              }
            },
            "gemini": {
              "baseurl": "https://generativelanguage.googleapis.com",
              "apikey": "YOUR_GEMINI_KEY",
              "authorization": "x-goog-api-key",
              "capability": {
                "models": {
                  "gemini-2.5-flash": {},
                  "gemini-2.5-pro": {}
                }
              },
              "name": "Google Gemini",
              "compatibility": {
                "openai_chat": false,
                "gemini_generate_content": true
              }
            },
            "vertex": {
              "baseurl": "https://aiplatform.googleapis.com",
              "authorization": "bearer",
              "apikey": "keyfile::ba3..3kb.data...67",
              "capability": {
                "models": {
                  "gemini-2.0-flash-exp": {},
                  "gemini-2.5-flash": {},
                  "gemini-2.5-flash-image": {},
                  "gemini-2.5-pro": {},
                  "claude-opus-4-5@20251101": {},
                  "claude-haiku-4-5@20251001": {},
                  "claude-sonnet-4-5@20250929": {},
                  "claude-opus-4-6": {}
                }
              },
              "compatibility": {
                // Gemini model support
                "google_generate_content": true,
                // Anthropic via Vertex model support
                "google_raw_predict": true,
              }
            },
            "openrouter": {
              "baseurl": "https://openrouter.ai/api/",
              "apikey": "YOUR_OPENROUTER_KEY",
              "capability": {
                "models": {
                  "qwen/qwen3-235b-a22b-2507": {},
                  "google/gemini-2.5-pro-preview": {},
                  "x-ai/grok-code-fast-1": {}
                }
              }
            },
            "private": {
              "baseurl": "YOUR_PRIVATE_LLM_URL",
              "tailnet": true,
              "capability": {
                "models": {
                  "qwen3-coder-30b": {},
                  "llama-3.1-70b": {}
                }
              }
            }
          }
        }
        "#;
        let config: Config = json5::from_str(json).unwrap();
        assert_eq!(config.providers.len(), 7);

        // Serialize to JSON, then deserialize again and assert equality
        let serialized = serde_json::to_string(&config).unwrap();
        let roundtripped: Config = json5::from_str(&serialized).unwrap();
        assert_eq!(config, roundtripped);
    }

    #[test]
    fn bad_log_level_is_rejected() {
        let json = r#"{
          "lacuna": {
            "logging": {
              "level": "superverbose"
            }
          },
          "providers": {}
        }"#;
        let err = Config::from_str(json).unwrap_err().to_string();
        assert!(
            err.contains("superverbose"),
            "expected error to mention the invalid value, got: {err}"
        );
    }
}
