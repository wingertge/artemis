use crate::{
    types::{HeaderPair, Middleware, Operation},
    Response
};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt};
use crate::types::{MiddlewareFactory, OperationResult};

#[derive(Debug)]
enum FetchError {
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

struct FetchMiddleware<TNext: Middleware + Send + Sync> {
    next: TNext
}

impl <TNext: Middleware + Send + Sync> MiddlewareFactory<TNext> for FetchMiddleware<TNext> {
    fn build(next: TNext) -> Self {
        Self {
            next
        }
    }
}

#[async_trait]
impl<TNext: Middleware + Send + Sync> Middleware for FetchMiddleware<TNext> {
    async fn run<T, F>(&self, operation: Operation<T, F>) -> Result<OperationResult, Box<dyn Error>>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: Fn() -> Vec<HeaderPair> + Send + Sync
    {
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
