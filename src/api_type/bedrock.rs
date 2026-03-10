use super::ResponseMetadata;

use super::ApiTypeHandler;

pub struct BedrockModelInvokeHandler;

impl ApiTypeHandler for BedrockModelInvokeHandler {
    fn id(&self) -> &'static str {
        "bedrock_model_invoke"
    }

    fn inspect_response(
        &self,
        _response: &http::Response<bytes::Bytes>,
    ) -> Result<ResponseMetadata, anyhow::Error> {
        Ok(ResponseMetadata::default())
    }
}
