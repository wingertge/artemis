use crate::{
    exchanges::Client,
    types::{ExchangeResult, Operation, OperationResult},
    DebugInfo, Exchange, ExchangeFactory, GraphQLQuery, HeaderPair, Response, ResultSource
};
use std::{error::Error, fmt, sync::Arc};
use surf::http::header::HeaderName;

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

impl<TNext: Exchange> ExchangeFactory<TNext> for FetchExchange {
    type Output = FetchExchange;

    fn build(self, _next: TNext) -> Self::Output {
        FetchExchange
    }
}

#[async_trait]
impl Exchange for FetchExchange {
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        operation: Operation<Q::Variables>,
        _client: C
    ) -> ExchangeResult<Q::ResponseData> {
        let extra_headers = if let Some(extra_headers) = operation.options.extra_headers {
            extra_headers()
        } else {
            Vec::new()
        };

        let mut request = surf::post(operation.options.url)
            .set_header("Content-Type", "application/json")
            .set_header("Accept", "application/json")
            .body_json(&operation.query)?;

        for HeaderPair(key, value) in extra_headers {
            let header_name: HeaderName = key.parse().unwrap();
            request = request.set_header(header_name, value);
        }

        let mut response: Response<Q::ResponseData> = request
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
            key: operation.key,
            meta: operation.meta,
            response
        })
    }
}
