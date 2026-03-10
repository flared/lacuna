use super::ApiTypeHandler;

pub struct BedrockModelInvokeHandler;

impl ApiTypeHandler for BedrockModelInvokeHandler {
    fn name(&self) -> &'static str {
        "Bedrock Model Invoke"
    }
}
