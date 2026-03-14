use super::super::{ApiTypeHandler, Inspector, MetadataInspector, ResponseMetadata};
use crate::inspector::protocol::ProtocolInspector;
use crate::inspector::protocol::text::{TextBody, TextProtocol};
use serde::Deserialize;

// https://platform.openai.com/docs/api-reference/chat/object
#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionResponse {
    usage: OpenAiChatCompletionUsage,
}

pub struct OpenAiChatCompletionHandler;

impl ApiTypeHandler for OpenAiChatCompletionHandler {
    fn id(&self) -> &'static str {
        "openai_chat_completion"
    }

    fn inspector(&self, _status: u16, _headers: &http::HeaderMap) -> MetadataInspector {
        Box::new(ProtocolInspector::new(
            TextProtocol::new(),
            OpenAiChatInspector { metadata: None },
        ))
    }
}

struct OpenAiChatInspector {
    metadata: Option<Result<ResponseMetadata, anyhow::Error>>,
}

impl Inspector<TextBody> for OpenAiChatInspector {
    type Output = ResponseMetadata;

    fn feed(&mut self, body: TextBody) {
        self.metadata = Some(parse_chat_completion(&body.data));
    }

    fn finish(self: Box<Self>) -> Result<ResponseMetadata, anyhow::Error> {
        self.metadata
            .unwrap_or_else(|| Err(anyhow::anyhow!("no response body")))
    }
}

fn parse_chat_completion(data: &[u8]) -> Result<ResponseMetadata, anyhow::Error> {
    let parsed = serde_json::from_slice::<OpenAiChatCompletionResponse>(data)?;
    Ok(ResponseMetadata {
        input_tokens: Some(parsed.usage.prompt_tokens),
        output_tokens: Some(parsed.usage.completion_tokens),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_inspector() -> MetadataInspector {
        OpenAiChatCompletionHandler.inspector(200, &http::HeaderMap::new())
    }

    #[test]
    fn inspect_response_full() {
        let body = br#"{
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "Hi!"}}],
            "usage": {"prompt_tokens": 15, "completion_tokens": 42, "total_tokens": 57}
        }"#;
        let mut inspector = make_inspector();
        inspector.feed(body);
        let metadata = inspector.finish().unwrap();
        assert_eq!(metadata.input_tokens, Some(15));
        assert_eq!(metadata.output_tokens, Some(42));
    }

    #[test]
    fn inspect_response_missing_usage() {
        let mut inspector = make_inspector();
        inspector.feed(br#"{"id": "chatcmpl-abc123", "object": "chat.completion"}"#);
        assert!(inspector.finish().is_err());
    }

    #[test]
    fn inspect_response_invalid_json() {
        let mut inspector = make_inspector();
        inspector.feed(b"not json");
        assert!(inspector.finish().is_err());
    }
}
