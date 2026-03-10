use serde::{Deserialize, Serialize};

use crate::api_type::{ApiType, api_type_for_path};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct Compatibility {
    #[serde(default)]
    pub openai_chat: bool,
    #[serde(default)]
    pub openai_responses: bool,
    #[serde(default)]
    pub anthropic_messages: bool,
    #[serde(default)]
    pub gemini_generate_content: bool,
    #[serde(default)]
    pub bedrock_model_invoke: bool,
    #[serde(default)]
    pub google_generate_content: bool,
    #[serde(default)]
    pub google_raw_predict: bool,
}

impl Compatibility {
    pub fn is_compatible(&self, path: &str) -> bool {
        match api_type_for_path(path) {
            Some(ApiType::OpenAiChat) => self.openai_chat,
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
}
