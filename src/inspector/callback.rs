use super::{ByteInspector, Inspector};

/// A wrapper that decorates any byte-level `Inspector`, calling a callback with the result on `finish()`.
pub struct CallbackInspector<T, Cb>
where
    Cb: FnOnce(&Result<T, anyhow::Error>) + Send + 'static,
{
    inner: ByteInspector<T>,
    callback: Option<Cb>,
}

impl<T, Cb> std::fmt::Debug for CallbackInspector<T, Cb>
where
    Cb: FnOnce(&Result<T, anyhow::Error>) + Send + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackInspector").finish_non_exhaustive()
    }
}

impl<T, Cb> CallbackInspector<T, Cb>
where
    Cb: FnOnce(&Result<T, anyhow::Error>) + Send + 'static,
{
    pub fn new(inner: ByteInspector<T>, callback: Cb) -> Self {
        Self {
            inner,
            callback: Some(callback),
        }
    }
}

impl<T, Cb> Inspector<&[u8]> for CallbackInspector<T, Cb>
where
    T: Send,
    Cb: FnOnce(&Result<T, anyhow::Error>) + Send + 'static,
{
    type Output = T;

    fn feed(&mut self, chunk: &[u8]) {
        self.inner.feed(chunk);
    }

    fn finish(self: Box<Self>) -> Result<T, anyhow::Error> {
        let this = *self;
        let result = this.inner.finish();
        if let Some(callback) = this.callback {
            callback(&result);
        }
        result
    }
}
