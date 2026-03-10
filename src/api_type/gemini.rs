use super::ApiTypeHandler;

pub struct GeminiGenerateContentHandler;

impl ApiTypeHandler for GeminiGenerateContentHandler {
    fn name(&self) -> &'static str {
        "Gemini Generate Content"
    }
}
