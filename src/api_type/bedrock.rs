use super::{ApiTypeHandler, MetadataInspector, ResponseMetadata, StaticInspector};

pub struct BedrockModelInvokeHandler;

impl ApiTypeHandler for BedrockModelInvokeHandler {
    fn id(&self) -> &'static str {
        "bedrock_model_invoke"
    }

    fn inspector(&self, _status: u16, headers: &http::HeaderMap) -> MetadataInspector {
        let input_tokens =
            parse_token_header(headers, "x-amzn-bedrock-input-token-count").unwrap_or(None);
        let output_tokens =
            parse_token_header(headers, "x-amzn-bedrock-output-token-count").unwrap_or(None);
        Box::new(StaticInspector::new(ResponseMetadata {
            input_tokens,
            output_tokens,
        }))
    }
}

fn parse_token_header(
    headers: &http::HeaderMap,
    header: &str,
) -> Result<Option<u64>, anyhow::Error> {
    match headers.get(header) {
        Some(value) => Ok(Some(value.to_str()?.parse::<u64>()?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_to_map(headers: &[(&str, &str)]) -> http::HeaderMap {
        let mut map = http::HeaderMap::new();
        for (key, value) in headers {
            map.insert(
                http::header::HeaderName::from_bytes(key.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        map
    }

    #[test]
    fn inspect_response_with_headers() {
        let headers = headers_to_map(&[
            ("x-amzn-bedrock-input-token-count", "25"),
            ("x-amzn-bedrock-output-token-count", "150"),
        ]);
        let inspector = BedrockModelInvokeHandler.inspector(200, &headers);
        let metadata = inspector.finish().unwrap();
        assert_eq!(metadata.input_tokens, Some(25));
        assert_eq!(metadata.output_tokens, Some(150));
    }

    #[test]
    fn inspect_response_missing_headers() {
        let headers = http::HeaderMap::new();
        let inspector = BedrockModelInvokeHandler.inspector(200, &headers);
        let metadata = inspector.finish().unwrap();
        assert_eq!(metadata.input_tokens, None);
        assert_eq!(metadata.output_tokens, None);
    }

    #[test]
    fn inspect_response_invalid_header() {
        let headers = headers_to_map(&[("x-amzn-bedrock-input-token-count", "not_a_number")]);
        let inspector = BedrockModelInvokeHandler.inspector(200, &headers);
        let metadata = inspector.finish().unwrap();
        // Invalid headers are silently ignored (unwrap_or(None) in inspector())
        assert_eq!(metadata.input_tokens, None);
        assert_eq!(metadata.output_tokens, None);
    }
}
