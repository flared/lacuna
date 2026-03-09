use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct Compatibility {
    pub openai_chat: bool,
    pub openai_responses: bool,
    pub anthropic_messages: bool,
    pub gemini_generate_content: bool,
    pub bedrock_model_invoke: bool,
    pub google_generate_content: bool,
    pub google_raw_predict: bool,
}

static RE_OPENAI_CHAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/v1/chat/completions").unwrap());
static RE_OPENAI_RESPONSES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/v1/responses").unwrap());
static RE_ANTHROPIC_MESSAGES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/v1/messages").unwrap());
static RE_GEMINI_GENERATE_CONTENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":generateContent$").unwrap());
static RE_BEDROCK_MODEL_INVOKE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/model/.+/invoke").unwrap());
static RE_GOOGLE_GENERATE_CONTENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":generateContent$").unwrap());
static RE_GOOGLE_RAW_PREDICT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":rawPredict$").unwrap());

impl Compatibility {
    pub fn is_compatible(&self, path: &str) -> bool {
        (self.openai_chat && RE_OPENAI_CHAT.is_match(path))
            || (self.openai_responses && RE_OPENAI_RESPONSES.is_match(path))
            || (self.anthropic_messages && RE_ANTHROPIC_MESSAGES.is_match(path))
            || (self.gemini_generate_content && RE_GEMINI_GENERATE_CONTENT.is_match(path))
            || (self.bedrock_model_invoke && RE_BEDROCK_MODEL_INVOKE.is_match(path))
            || (self.google_generate_content && RE_GOOGLE_GENERATE_CONTENT.is_match(path))
            || (self.google_raw_predict && RE_GOOGLE_RAW_PREDICT.is_match(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_chat() {
        let c = Compatibility {
            openai_chat: true,
            ..Default::default()
        };
        assert!(c.is_compatible("/v1/chat/completions"));
        assert!(c.is_compatible("/v1/chat/completions?stream=true"));
        assert!(!c.is_compatible("/v1/responses"));
    }

    #[test]
    fn openai_responses() {
        let c = Compatibility {
            openai_responses: true,
            ..Default::default()
        };
        assert!(c.is_compatible("/v1/responses"));
        assert!(c.is_compatible("/v1/responses/resp_123"));
        assert!(!c.is_compatible("/v1/chat/completions"));
    }

    #[test]
    fn anthropic_messages() {
        let c = Compatibility {
            anthropic_messages: true,
            ..Default::default()
        };
        assert!(c.is_compatible("/v1/messages"));
        assert!(c.is_compatible("/v1/messages?stream=true"));
        assert!(!c.is_compatible("/v1/chat/completions"));
    }

    #[test]
    fn gemini_generate_content() {
        let c = Compatibility {
            gemini_generate_content: true,
            ..Default::default()
        };
        assert!(c.is_compatible("/v1/models/gemini-2.0-flash:generateContent"));
        assert!(!c.is_compatible("/v1/chat/completions"));
    }

    #[test]
    fn bedrock_model_invoke() {
        let c = Compatibility {
            bedrock_model_invoke: true,
            ..Default::default()
        };
        assert!(c.is_compatible("/model/us.anthropic.claude-sonnet-4-5/invoke"));
        assert!(!c.is_compatible("/v1/chat/completions"));
    }

    #[test]
    fn google_generate_content() {
        let c = Compatibility {
            google_generate_content: true,
            ..Default::default()
        };
        assert!(c.is_compatible("/v1/projects/my-proj/locations/us/publishers/google/models/gemini-2.5-pro:generateContent"));
        assert!(!c.is_compatible("/v1/chat/completions"));
    }

    #[test]
    fn google_raw_predict() {
        let c = Compatibility {
            google_raw_predict: true,
            ..Default::default()
        };
        assert!(c.is_compatible(
            "/v1/projects/my-proj/locations/us/publishers/google/models/claude-opus-4-5:rawPredict"
        ));
        assert!(!c.is_compatible("/v1/chat/completions"));
    }

    #[test]
    fn no_flags_matches_nothing() {
        let c = Compatibility::default();
        assert!(!c.is_compatible("/v1/chat/completions"));
        assert!(!c.is_compatible("/v1/messages"));
        assert!(!c.is_compatible("/anything"));
    }

    #[test]
    fn unrelated_path_matches_nothing() {
        let c = Compatibility {
            openai_chat: true,
            ..Default::default()
        };
        assert!(!c.is_compatible("/health"));
        assert!(!c.is_compatible("/v2/something"));
    }
}
