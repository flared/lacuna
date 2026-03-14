use super::ApiTypeHandler;

pub struct GoogleGenerateContentHandler;

impl ApiTypeHandler for GoogleGenerateContentHandler {
    fn id(&self) -> &'static str {
        "google_generate_content"
    }
}

pub struct GoogleRawPredictHandler;

impl ApiTypeHandler for GoogleRawPredictHandler {
    fn id(&self) -> &'static str {
        "google_raw_predict"
    }
}
