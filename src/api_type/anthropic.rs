use super::ApiTypeHandler;

pub struct AnthropicMessagesHandler;

impl ApiTypeHandler for AnthropicMessagesHandler {
    fn name(&self) -> &'static str {
        "Anthropic Messages"
    }
}
