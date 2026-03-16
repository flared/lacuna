use serde::Deserialize;

use crate::inspector::protocol::amazon_eventstream::EventstreamEvent;

use super::super::anthropic::AnthropicSseInspector;
use super::super::{Inspector, ResponseMetadata};

/// Bedrock wraps each inner JSON payload in `{"bytes":"<base64>"}`.
#[derive(Deserialize)]
struct BedrockEnvelope {
    bytes: String,
}

impl Inspector<EventstreamEvent> for AnthropicSseInspector {
    type Output = ResponseMetadata;

    fn feed(&mut self, event: EventstreamEvent) {
        if let Ok(envelope) = serde_json::from_slice::<BedrockEnvelope>(&event.payload)
            && let Ok(decoded) = aws_smithy_types::base64::decode(&envelope.bytes)
            && let Ok(json_str) = std::str::from_utf8(&decoded)
        {
            self.process_event_json(json_str);
        }
    }

    fn finish(self: Box<Self>) -> Result<ResponseMetadata, anyhow::Error> {
        if self.input_tokens.is_none() && self.output_tokens.is_none() {
            return Err(anyhow::anyhow!("no token usage found in SSE stream"));
        }
        let response_metadata = ResponseMetadata {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
        };
        Ok(response_metadata)
    }
}

#[cfg(test)]
mod tests {
    use crate::api_type::anthropic::AnthropicSseInspector;
    use crate::inspector::protocol::ProtocolInspector;
    use crate::inspector::protocol::amazon_eventstream::AmazonEventstreamProtocol;
    use crate::inspector::protocol::amazon_eventstream::testutil::build_eventstream_frame;

    fn wrap_anthropic_event(json: &str) -> Vec<u8> {
        let b64 = aws_smithy_types::base64::encode(json.as_bytes());
        let payload = serde_json::json!({"bytes": b64, "p": "abc"});
        build_eventstream_frame(payload.to_string().as_bytes())
    }

    #[test]
    fn inspect_eventstream_response() {
        let mut inspector: super::super::super::ResponseMetadataInspector =
            Box::new(ProtocolInspector::new(
                AmazonEventstreamProtocol::default(),
                AnthropicSseInspector {
                    input_tokens: None,
                    output_tokens: None,
                },
            ));

        // message_start with input_tokens
        let frame1 = wrap_anthropic_event(
            r#"{"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","usage":{"input_tokens":25,"output_tokens":1}}}"#,
        );
        inspector.feed(&frame1);

        // content_block_delta (ignored)
        let frame2 = wrap_anthropic_event(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi!"}}"#,
        );
        inspector.feed(&frame2);

        // message_delta with output_tokens
        let frame3 = wrap_anthropic_event(
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":150}}"#,
        );
        inspector.feed(&frame3);

        // message_stop
        let frame4 = wrap_anthropic_event(
            r#"{"type":"message_stop","amazon-bedrock-invocationMetrics":{"inputTokenCount":25,"outputTokenCount":150}}"#,
        );
        inspector.feed(&frame4);

        let metadata = inspector.finish().unwrap();
        assert_eq!(metadata.input_tokens, Some(25));
        assert_eq!(metadata.output_tokens, Some(150));
    }
}
