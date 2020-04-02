use crate::{Exchange, ExchangeFactory, Operation, OperationResult, OperationType};
use futures::Stream;
use std::{collections::HashMap, error::Error};
use serde::Serialize;
use crate::types::ExchangeResult;

pub trait SubscriptionSource: Send + Sync {
    fn subscribe<T>(
        &self,
        operation: Operation<T>
    ) -> Result<Box<dyn Stream<Item = Result<SubscriptionResult, Box<dyn Error>>>>, Box<dyn Error>>;
}

pub struct SubscriptionResult {
    result: OperationResult,
    extensions: HashMap<String, serde_json::Value>
}

pub struct SubscriptionExchange<T: SubscriptionSource> {
    source: T
}

impl<T: SubscriptionSource> SubscriptionExchange<T> {
    pub fn new(source: T) -> Self {
        Self { source }
    }
}

impl<T, TNext> ExchangeFactory<SubscriptionExchangeImpl<T, TNext>, TNext>
    for SubscriptionExchange<T>
where
    T: SubscriptionSource,
    TNext: Exchange
{
    fn build(self, next: TNext) -> SubscriptionExchangeImpl<T, TNext> {
        SubscriptionExchangeImpl {
            source: self.source,
            next
        }
    }
}

pub(crate) struct SubscriptionExchangeImpl<T, TNext>
where
    T: SubscriptionSource,
    TNext: Exchange
{
    source: T,
    next: TNext
}

impl<T, TNext> Exchange for SubscriptionExchangeImpl<T, TNext>
where
    T: SubscriptionSource,
    TNext: Exchange
{
    async fn run<V: Serialize + Send + Sync>(
        &self,
        operation: Operation<V>
    ) -> ExchangeResult {
        if &operation.meta.operation_type != &OperationType::Subscription {
            return self.next.run(operation);
        }

        let subscription_source = self.source.subscribe(operation)?;

    }
}
