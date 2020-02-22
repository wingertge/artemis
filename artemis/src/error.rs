use std::sync::Arc;
use std::fmt;
use std::error::Error;

#[derive(Clone, Debug)]
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


impl QueryError {
    pub fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }

    pub fn compat(self) -> QueryErrorCompat {
        QueryErrorCompat(self)
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<T: Error + Send + Sync + 'static> From<T> for QueryError {
    fn from(e: T) -> Self {
        QueryError {
            inner: Arc::new(Box::new(e))
        }
    }
}