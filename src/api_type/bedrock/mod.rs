mod eventstream_inspector;
mod headers_inspector;

use crate::inspector::protocol::ProtocolInspector;
use crate::inspector::protocol::amazon_eventstream::AmazonEventstreamProtocol;

use super::anthropic::AnthropicSseInspector;
use super::{ApiTypeHandler, MetadataInspector};

pub struct BedrockModelInvokeHandler;

fn is_amazon_event_stream(headers: &http::HeaderMap) -> bool {
    headers
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("application/vnd.amazon.eventstream"))
}

impl ApiTypeHandler for BedrockModelInvokeHandler {
    fn id(&self) -> &'static str {
        "bedrock_model_invoke"
    }

    fn inspector(&self, status: u16, headers: &http::HeaderMap) -> MetadataInspector {
        if is_amazon_event_stream(headers) {
            Box::new(ProtocolInspector::new(
                AmazonEventstreamProtocol::default(),
                AnthropicSseInspector {
                    input_tokens: None,
                    output_tokens: None,
                },
            ))
        } else {
            headers_inspector::BedrockModelInvokeJsonHandler.inspector(status, headers)
        }
    }
}
