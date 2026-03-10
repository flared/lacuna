use super::ApiTypeHandler;

pub struct OpenAiChatHandler;

impl ApiTypeHandler for OpenAiChatHandler {
    fn name(&self) -> &'static str {
        "OpenAI Chat"
    }
}

pub struct OpenAiResponsesHandler;

impl ApiTypeHandler for OpenAiResponsesHandler {
    fn name(&self) -> &'static str {
        "OpenAI Responses"
    }
}
