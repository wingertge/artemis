use std::{error::Error, fmt, sync::Arc};

/// A query error wrapper that allows for cheap and easy cloning across threads.
/// If a `std::error::Error` is needed, use `QueryError.compat()`.
#[derive(Clone, Debug)]
pub struct QueryError {
    inner: Arc<Box<dyn Error + Send + Sync>>
}

impl PartialEq for QueryError {
    /// This is just for testing. Wrapped Errors can't reasonably be compared,
    /// so this will always return false.
    fn eq(&self, _other: &Self) -> bool {
        false
    }
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
    /// Gets the source of the inner error
    pub fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }

    /// Gets a compatibility wrapper that implements `std::error::Error`. This is necessary until specialization lands.
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
