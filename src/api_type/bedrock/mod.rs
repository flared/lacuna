mod eventstream_inspector;
mod headers_inspector;

use crate::inspector::protocol::ProtocolInspector;
use crate::inspector::protocol::amazon_eventstream::AmazonEventstreamProtocol;

use super::anthropic::AnthropicSseInspector;
use super::{ApiTypeHandler, ResponseMetadataInspector};
use crate::model_rewrite::ResolvedModelRewrite;
use crate::request_metadata::RequestInspectionMetadata;

pub struct BedrockModelInvokeHandler;

use async_trait::async_trait;

fn is_amazon_event_stream(headers: &http::HeaderMap) -> bool {
    headers
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("application/vnd.amazon.eventstream"))
}

/// Extract the model ID from a Bedrock path like `/model/<model_id>/invoke` or
/// `/model/<model_id>/invokeWithResponseStream`.
fn extract_model_from_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/model/")?;
    let model = rest.split('/').next()?;
    if model.is_empty() {
        return None;
    }
    Some(model.to_owned())
}

#[async_trait]
impl ApiTypeHandler for BedrockModelInvokeHandler {
    fn id(&self) -> &'static str {
        "bedrock_model_invoke"
    }

    async fn inspect_request(
        &self,
        request: axum::extract::Request,
    ) -> (
        anyhow::Result<RequestInspectionMetadata>,
        axum::extract::Request,
    ) {
        let model = extract_model_from_path(request.uri().path());
        (
            Ok(RequestInspectionMetadata {
                model,
                ..Default::default()
            }),
            request,
        )
    }

    async fn rewrite_model_in_request(
        &self,
        request: axum::extract::Request,
        rewrite: &ResolvedModelRewrite,
    ) -> anyhow::Result<axum::extract::Request> {
        crate::model_rewrite::rewrite_request_path(request, rewrite)
    }

    fn response_inspector(
        &self,
        status: u16,
        headers: &http::HeaderMap,
        request_metadata: &RequestInspectionMetadata,
    ) -> ResponseMetadataInspector {
        if is_amazon_event_stream(headers) {
            Box::new(ProtocolInspector::new(
                AmazonEventstreamProtocol::default(),
                AnthropicSseInspector {
                    input_tokens: None,
                    output_tokens: None,
                    cache_creation_tokens: None,
                    cache_read_input_tokens: None,
                    cache_ttl_hint: request_metadata.cache_ttl_secs,
                },
            ))
        } else {
            headers_inspector::BedrockModelInvokeJsonHandler.response_inspector(
                status,
                headers,
                request_metadata,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_model_from_invoke_path() {
        assert_eq!(
            extract_model_from_path("/model/us.anthropic.claude-sonnet-4-5/invoke"),
            Some("us.anthropic.claude-sonnet-4-5".to_owned()),
        );
    }

    #[test]
    fn extract_model_from_streaming_path() {
        assert_eq!(
            extract_model_from_path(
                "/model/us.anthropic.claude-opus-4-5-20251101-v1:0/invokeWithResponseStream"
            ),
            Some("us.anthropic.claude-opus-4-5-20251101-v1:0".to_owned()),
        );
    }

    #[test]
    fn extract_model_from_invalid_paths() {
        assert_eq!(extract_model_from_path("/model//invoke"), None);
        assert_eq!(extract_model_from_path("/other/path"), None);
        assert_eq!(extract_model_from_path(""), None);
    }
}
