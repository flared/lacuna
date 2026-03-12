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
    pub fn is_compatible(&self, path: &str) -> bool {
        match api_type_for_path(path) {
            Some(ApiType::OpenAiChatCompletion) => self.openai_chat,
            Some(ApiType::OpenAiResponses) => self.openai_responses,
            Some(ApiType::AnthropicMessages) => self.anthropic_messages,
            Some(ApiType::GeminiGenerateContent) => self.gemini_generate_content,
            Some(ApiType::BedrockModelInvoke) => self.bedrock_model_invoke,
            Some(ApiType::GoogleGenerateContent) => self.google_generate_content,
            Some(ApiType::GoogleRawPredict) => self.google_raw_predict,
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
        assert!(!c.is_compatible("/v1/chat/completions"));
        assert!(!c.is_compatible("/v1/messages"));
        assert!(!c.is_compatible("/anything"));
    }

    #[test]
    fn disabled_flag_rejects_matching_path() {
        let c = Compatibility {
            openai_chat: false,
            anthropic_messages: true,
            ..Default::default()
        };
        assert!(!c.is_compatible("/v1/chat/completions"));
        assert!(c.is_compatible("/v1/messages"));
    }

    #[test]
    fn multiple_flags_enabled() {
        let c = Compatibility {
            openai_chat: true,
            anthropic_messages: true,
            bedrock_model_invoke: true,
            ..Default::default()
        };
        assert!(c.is_compatible("/v1/chat/completions"));
        assert!(c.is_compatible("/v1/messages"));
        assert!(c.is_compatible("/model/us.anthropic.claude-sonnet-4-5/invoke"));
        assert!(!c.is_compatible("/v1/responses"));
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
