use super::super::{ApiTypeHandler, Inspector, ResponseMetadata, ResponseMetadataInspector};
use crate::inspector::protocol::ProtocolInspector;
use crate::inspector::protocol::text::{TextBody, TextProtocol};
use serde::Deserialize;

// https://platform.openai.com/docs/api-reference/responses/object
#[derive(Debug, Deserialize)]
struct OpenAiResponsesUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesResponse {
    usage: OpenAiResponsesUsage,
}

pub struct OpenAiResponsesHandler;

impl ApiTypeHandler for OpenAiResponsesHandler {
    fn id(&self) -> &'static str {
        "openai_responses"
    }

    fn response_inspector(
        &self,
        _status: u16,
        _headers: &http::HeaderMap,
    ) -> ResponseMetadataInspector {
        Box::new(ProtocolInspector::new(
            TextProtocol::new(),
            OpenAiResponsesInspector { metadata: None },
        ))
    }
}

struct OpenAiResponsesInspector {
    metadata: Option<Result<ResponseMetadata, anyhow::Error>>,
}

impl Inspector<TextBody> for OpenAiResponsesInspector {
    type Output = ResponseMetadata;

    fn feed(&mut self, body: TextBody) {
        self.metadata = Some(parse_responses(&body.data));
    }

    fn finish(self: Box<Self>) -> Result<ResponseMetadata, anyhow::Error> {
        self.metadata
            .unwrap_or_else(|| Err(anyhow::anyhow!("no response body")))
    }
}

fn parse_responses(data: &[u8]) -> Result<ResponseMetadata, anyhow::Error> {
    let parsed = serde_json::from_slice::<OpenAiResponsesResponse>(data)?;
    Ok(ResponseMetadata {
        input_tokens: Some(parsed.usage.input_tokens),
        output_tokens: Some(parsed.usage.output_tokens),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_inspector() -> ResponseMetadataInspector {
        OpenAiResponsesHandler.response_inspector(200, &http::HeaderMap::new())
    }

    #[test]
    fn inspect_response_full() {
        let body = br#"{
            "id": "resp_abc123",
            "object": "response",
            "model": "gpt-4o",
            "usage": {"input_tokens": 10, "output_tokens": 30, "total_tokens": 40}
        }"#;
        let mut inspector = make_inspector();
        inspector.feed(body);
        let metadata = inspector.finish().unwrap();
        assert_eq!(metadata.input_tokens, Some(10));
        assert_eq!(metadata.output_tokens, Some(30));
    }

    #[test]
    fn inspect_response_missing_usage() {
        let mut inspector = make_inspector();
        inspector.feed(br#"{"id": "resp_abc123", "object": "response"}"#);
        assert!(inspector.finish().is_err());
    }

    #[test]
    fn inspect_response_invalid_json() {
        let mut inspector = make_inspector();
        inspector.feed(b"not json");
        assert!(inspector.finish().is_err());
    }
}
