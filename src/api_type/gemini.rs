use super::ResponseMetadata;

use super::ApiTypeHandler;

pub struct GeminiGenerateContentHandler;

impl ApiTypeHandler for GeminiGenerateContentHandler {
    fn id(&self) -> &'static str {
        "gemini_generate_content"
    }

    fn inspect_response(
        &self,
        _response: &http::Response<bytes::Bytes>,
    ) -> Result<ResponseMetadata, anyhow::Error> {
        Ok(ResponseMetadata::default())
    }
}
