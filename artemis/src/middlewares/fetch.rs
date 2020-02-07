use crate::types::{HeaderPair, Middleware, MiddlewareFactory, Operation, OperationResult};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum FetchError {
    FetchError(Box<dyn Error + Send + Sync>),
    DecodeError(Box<dyn Error + Send + Sync>)
}
impl Error for FetchError {}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::FetchError(e) => write!(f, "fetch error: {}", e),
            FetchError::DecodeError(e) => write!(f, "decoding error: {}", e)
        }
    }
}

pub struct FetchMiddleware<TNext: Middleware + Send + Sync> {
    next: TNext
}

impl<TNext: Middleware + Send + Sync> MiddlewareFactory<FetchMiddleware<TNext>, TNext>
    for FetchMiddleware<TNext>
{
    fn build(next: TNext) -> Self {
        Self { next }
    }
}

#[async_trait]
impl<TNext: Middleware + Send + Sync> Middleware for FetchMiddleware<TNext> {
    async fn run<V: Serialize + Send + Sync>(
        &self,
        operation: Operation<V>
    ) -> Result<OperationResult, Box<dyn Error>> {
        let extra_headers = if let Some(extra_headers) = operation.extra_headers {
            extra_headers()
        } else {
            Vec::new()
        };

        let mut request = surf::post(operation.url)
            .set_header("Content-Type", "application/json")
            .set_header("Accept", "application/json")
            .body_json(&operation.query)?;

        for HeaderPair(key, value) in extra_headers {
            request = request.set_header(key, value)
        }

        let response = request
            .await
            .map_err(FetchError::FetchError)?
            .body_string()
            .await
            .map_err(FetchError::DecodeError)?;

        Ok(OperationResult {
            response_string: response
        })
    }
}
