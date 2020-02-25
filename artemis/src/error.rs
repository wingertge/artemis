use std::{error::Error, fmt, sync::Arc};

#[derive(Clone, Debug)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct QueryError {
    inner: Arc<Box<dyn Error + Send + Sync>>
}

#[derive(Debug)]
pub struct QueryErrorCompat(QueryError);

impl Error for QueryErrorCompat {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

impl fmt::Display for QueryErrorCompat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl QueryError {
    pub fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }

    pub fn compat(self) -> QueryErrorCompat {
        QueryErrorCompat(self)
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl<T: Error + Send + Sync + 'static> From<T> for QueryError {
    fn from(e: T) -> Self {
        QueryError {
            inner: Arc::new(Box::new(e))
        }
    }
}
