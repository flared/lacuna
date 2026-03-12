use serde::{Deserialize, Serialize};

use crate::api_type::{ApiType, api_type_for_path};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Compatibility {
    pub openai_chat: bool,
    pub openai_responses: bool,
    pub anthropic_messages: bool,
    pub gemini_generate_content: bool,
    pub bedrock_model_invoke: bool,
    pub google_generate_content: bool,
    pub google_raw_predict: bool,
}

impl Compatibility {
    pub fn is_compatible(&self, api_type: &ApiType) -> bool {
        match api_type {
            ApiType::OpenAiChatCompletion => self.openai_chat,
            ApiType::OpenAiResponses => self.openai_responses,
            ApiType::AnthropicMessages => self.anthropic_messages,
            ApiType::GeminiGenerateContent => self.gemini_generate_content,
            ApiType::BedrockModelInvoke => self.bedrock_model_invoke,
            ApiType::GoogleGenerateContent => self.google_generate_content,
            ApiType::GoogleRawPredict => self.google_raw_predict,
        }
    }
    pub fn is_compatible_with_path(&self, path: &str) -> bool {
        match api_type_for_path(path) {
            Some(api_type) => self.is_compatible(&api_type),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_flags_matches_nothing() {
        let c = Compatibility::default();
        assert!(!c.is_compatible(&ApiType::OpenAiChatCompletion));
        assert!(!c.is_compatible(&ApiType::AnthropicMessages));
        assert!(!c.is_compatible(&ApiType::BedrockModelInvoke));
    }

    #[test]
    fn multiple_flags_enabled() {
        let c = Compatibility {
            openai_chat: true,
            anthropic_messages: true,
            bedrock_model_invoke: true,
            ..Default::default()
        };
        assert!(c.is_compatible(&ApiType::OpenAiChatCompletion));
        assert!(c.is_compatible(&ApiType::AnthropicMessages));
        assert!(c.is_compatible(&ApiType::BedrockModelInvoke));
        assert!(!c.is_compatible(&ApiType::OpenAiResponses));
    }

    #[test]
    fn deserialize_from_json() {
        let json = r#"{"openai_chat": true, "anthropic_messages": true}"#;
        let c: Compatibility = serde_json::from_str(json).unwrap();
        assert_eq!(
            c,
            Compatibility {
                openai_chat: true,
                anthropic_messages: true,
                gemini_generate_content: false,
                ..Default::default()
            }
        );
    }

    #[test]
    fn deserialize_from_json_unknown_fields_rejected() {
        let json = r#"{"openai_chat": true, "bogus_field": true}"#;
        let err = serde_json::from_str::<Compatibility>(json).unwrap_err();
        assert!(
            err.to_string().contains("bogus_field"),
            "expected error to mention unknown field, got: {err}"
        );
    }
}
