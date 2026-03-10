use super::ApiTypeHandler;

pub struct GoogleGenerateContentHandler;

impl ApiTypeHandler for GoogleGenerateContentHandler {
    fn name(&self) -> &'static str {
        "Google Generate Content"
    }
}

pub struct GoogleRawPredictHandler;

impl ApiTypeHandler for GoogleRawPredictHandler {
    fn name(&self) -> &'static str {
        "Google Raw Predict"
    }
}
