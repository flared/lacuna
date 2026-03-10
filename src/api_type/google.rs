use super::ResponseMetadata;

use super::ApiTypeHandler;

pub struct GoogleGenerateContentHandler;

impl ApiTypeHandler for GoogleGenerateContentHandler {
    fn id(&self) -> &'static str {
        "google_generate_content"
    }

    fn inspect_response(
        &self,
        _response: &http::Response<bytes::Bytes>,
    ) -> Result<ResponseMetadata, anyhow::Error> {
        Ok(ResponseMetadata::default())
    }
}

pub struct GoogleRawPredictHandler;

impl ApiTypeHandler for GoogleRawPredictHandler {
    fn id(&self) -> &'static str {
        "google_raw_predict"
    }

    fn inspect_response(
        &self,
        _response: &http::Response<bytes::Bytes>,
    ) -> Result<ResponseMetadata, anyhow::Error> {
        Ok(ResponseMetadata::default())
    }
}
