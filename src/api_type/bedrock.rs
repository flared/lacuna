use super::ApiTypeHandler;
use super::ResponseMetadata;

pub struct BedrockModelInvokeHandler;

impl ApiTypeHandler for BedrockModelInvokeHandler {
    fn id(&self) -> &'static str {
        "bedrock_model_invoke"
    }

    fn inspect_response(
        &self,
        response: &http::Response<bytes::Bytes>,
    ) -> Result<ResponseMetadata, anyhow::Error> {
        Ok(ResponseMetadata {
            input_tokens: parse_token_header(response, "x-amzn-bedrock-input-token-count")?,
            output_tokens: parse_token_header(response, "x-amzn-bedrock-output-token-count")?,
        })
    }
}

fn parse_token_header(
    response: &http::Response<bytes::Bytes>,
    header: &str,
) -> Result<Option<u64>, anyhow::Error> {
    match response.headers().get(header) {
        Some(value) => Ok(Some(value.to_str()?.parse::<u64>()?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn response_with_headers(headers: &[(&str, &str)]) -> http::Response<Bytes> {
        let mut builder = http::Response::builder().status(200);
        for (key, value) in headers {
            builder = builder.header(*key, *value);
        }
        builder.body(Bytes::new()).unwrap()
    }

    #[test]
    fn inspect_response_with_headers() {
        let response = response_with_headers(&[
            ("x-amzn-bedrock-input-token-count", "25"),
            ("x-amzn-bedrock-output-token-count", "150"),
        ]);
        let metadata = BedrockModelInvokeHandler
            .inspect_response(&response)
            .unwrap();
        assert_eq!(metadata.input_tokens, Some(25));
        assert_eq!(metadata.output_tokens, Some(150));
    }

    #[test]
    fn inspect_response_missing_headers() {
        let response = response_with_headers(&[]);
        let metadata = BedrockModelInvokeHandler
            .inspect_response(&response)
            .unwrap();
        assert_eq!(metadata.input_tokens, None);
        assert_eq!(metadata.output_tokens, None);
    }

    #[test]
    fn inspect_response_invalid_header() {
        let response =
            response_with_headers(&[("x-amzn-bedrock-input-token-count", "not_a_number")]);
        assert!(
            BedrockModelInvokeHandler
                .inspect_response(&response)
                .is_err()
        );
    }
}
