pub use crate::provider::compatibility::Compatibility;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Provider {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub description: String,

    pub baseurl: String,

    #[serde(default)]
    pub models: Vec<String>,

    #[serde(default)]
    pub apikey: String,

    #[serde(default)]
    pub authorization: Authorization,

    #[serde(default)]
    pub tailnet: bool,

    #[serde(default)]
    pub compatibility: Compatibility,
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

    #[test]
    fn deserialize_json_config() {
        let json = r#"{
          "providers": {
            "openai": {
              "name": "OpenAI",
              "baseurl": "https://api.openai.com/v1",
              "models": ["gpt-4o", "gpt-4o-mini"],
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
              "models": ["claude-sonnet-4-20250514"],
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
              "models": ["gemini-2.0-flash"],
              "authorization": "x-goog-api-key"
            }
          }
        }"#;
        let config: Config = json5::from_str(json).unwrap();
        assert_eq!(config.providers.len(), 3);

        let openai = &config.providers["openai"];
        assert_eq!(openai.name, "OpenAI");
        assert_eq!(openai.baseurl, "https://api.openai.com/v1");
        assert_eq!(openai.models, vec!["gpt-4o", "gpt-4o-mini"]);
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
        assert!(!gemini.compatibility.openai_responses);
    }

    #[test]
    fn defaults_applied() {
        let json = r#"{
          "providers": {
            "minimal": {
              "name": "Minimal",
              "baseurl": "https://example.com",
              "models": ["model-1"]
            }
          }
        }"#;
        let config: Config = json5::from_str(json).unwrap();
        let p = &config.providers["minimal"];
        assert_eq!(p.name, "Minimal");
        assert_eq!(p.apikey, "");
        assert_eq!(p.authorization, Authorization::None);
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
              "models": ["gpt-5", "gpt-5-mini", "gpt-4.1"],
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
              "models": [
                "us.anthropic.claude-haiku-4-5-20251001-v1:0",
                "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
                "us.anthropic.claude-opus-4-5-20251101-v1:0",
                "us.anthropic.claude-opus-4-6-v1"
              ],
              "compatibility": {
                "bedrock_model_invoke": true
              }
            },
            "anthropic": {
              "baseurl": "https://api.anthropic.com",
              "apikey": "YOUR_ANTHROPIC_KEY",
              "authorization": "x-api-key",
              "models": ["claude-sonnet-4-5", "claude-haiku-4-5", "claude-opus-4-5"],
              "compatibility": {
                "openai_chat": false,
                "anthropic_messages": true
              }
            },
            "gemini": {
              "baseurl": "https://generativelanguage.googleapis.com",
              "apikey": "YOUR_GEMINI_KEY",
              "authorization": "x-goog-api-key",
              "models": ["gemini-2.5-flash", "gemini-2.5-pro"],
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
              "models": [
                "gemini-2.0-flash-exp",
                "gemini-2.5-flash",
                "gemini-2.5-flash-image",
                "gemini-2.5-pro",
                "claude-opus-4-5@20251101",
                "claude-haiku-4-5@20251001",
                "claude-sonnet-4-5@20250929",
                "claude-opus-4-6"
              ],
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
              "models": [
                "qwen/qwen3-235b-a22b-2507",
                "google/gemini-2.5-pro-preview",
                "x-ai/grok-code-fast-1"
              ]
            },
            "private": {
              "baseurl": "YOUR_PRIVATE_LLM_URL",
              "tailnet": true,
              "models": ["qwen3-coder-30b", "llama-3.1-70b"]
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
