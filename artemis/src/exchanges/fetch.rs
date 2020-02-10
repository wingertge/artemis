use crate::{
    types::{Operation, OperationResult},
    DebugInfo, Exchange, ExchangeFactory, HeaderPair, Response, ResultSource
};
use serde::Serialize;
use std::{error::Error, fmt};
use crate::types::ExchangeResult;

#[derive(Debug)]
pub enum FetchError {
    FetchError(Box<dyn Error + Send + Sync>),
    DecodeError(std::io::Error)
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

pub struct FetchExchange;

impl<TNext: Exchange> ExchangeFactory<FetchExchange, TNext> for FetchExchange {
    fn build(_next: TNext) -> Self {
        Self {}
    }
}

#[async_trait]
impl Exchange for FetchExchange {
    async fn run<V: Serialize + Send + Sync>(
        &self,
        operation: Operation<V>
    ) -> ExchangeResult {
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

        let mut response: Response<serde_json::Value> = request
            .await
            .map_err(FetchError::FetchError)?
            .body_json()
            .await
            .map_err(FetchError::DecodeError)?;

        let debug_info = Some(DebugInfo {
            // TODO: Make this conditional
            source: ResultSource::Network,
            did_dedup: false
        });

        response.debug_info = debug_info;

        Ok(OperationResult {
            meta: operation.meta,
            response
        })
    }
}
