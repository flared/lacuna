use super::ApiTypeHandler;

pub struct GeminiGenerateContentHandler;

impl ApiTypeHandler for GeminiGenerateContentHandler {
    fn id(&self) -> &'static str {
        "gemini_generate_content"
    }
}
