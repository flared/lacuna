use super::Inspector;

/// An inspector that ignores all input and returns a pre-computed value.
#[derive(Debug, Default)]
pub struct StaticInspector<T> {
    value: T,
}

impl<T> StaticInspector<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<F, T: Send> Inspector<F> for StaticInspector<T> {
    type Output = T;

    fn feed(&mut self, _frame: F) {}

    fn finish(self: Box<Self>) -> Result<T, anyhow::Error> {
        Ok(self.value)
    }
}
