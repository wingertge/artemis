use crate::store::Store;
use artemis::{
    DebugInfo, Exchange, ExchangeFactory, ExchangeResult, GraphQLQuery, Operation, OperationResult,
    OperationType, RequestPolicy, Response, ResultSource
};
use std::{collections::HashMap, error::Error, sync::Arc};

#[derive(Default)]
pub struct NormalizedCacheExchange {
    options: Option<NormalizedCacheOptions>
}

impl NormalizedCacheExchange {
    #[allow(unused)]
    pub fn new(options: NormalizedCacheOptions) -> Self {
        Self {
            options: Some(options)
        }
    }
}

impl<TNext: Exchange> ExchangeFactory<NormalizedCacheImpl<TNext>, TNext>
    for NormalizedCacheExchange
{
    fn build(self, next: TNext) -> NormalizedCacheImpl<TNext> {
        let options = self
            .options
            .unwrap_or_else(|| NormalizedCacheOptions::default());
        let store = Store::new(options.custom_keys.unwrap_or_else(|| HashMap::new()));
        NormalizedCacheImpl {
            next,
            store: Arc::new(store)
        }
    }
}

pub(crate) struct NormalizedCacheImpl<TNext: Exchange> {
    next: TNext,
    store: Arc<Store>
}

#[derive(Default)]
pub struct NormalizedCacheOptions {
    custom_keys: Option<HashMap<&'static str, String>>
}

fn should_cache<Q: GraphQLQuery>(operation: &Operation<Q::Variables>) -> bool {
    operation.meta.operation_type == OperationType::Query
        && operation.request_policy != RequestPolicy::NetworkOnly
}

/*fn is_optimistic_mutation<T: QueryVariables>(op: &Operation<T>) -> bool {
    op.meta.operation_type == OperationType::Mutation
        && op.request_policy != RequestPolicy::NetworkOnly
}*/

impl<TNext: Exchange> NormalizedCacheImpl<TNext> {}

impl<TNext: Exchange> NormalizedCacheImpl<TNext> {
    fn after_query<Q: GraphQLQuery>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables
    ) -> Result<(), Box<dyn Error>> {
        self.store.write_query::<Q>(result, variables)
    }

    fn after_mutation<Q: GraphQLQuery>(&self, result: &OperationResult<Q::ResponseData>, variables: Q::Variables) {
        self.store.invalidate_query::<Q>(result, &variables);
    }
}

#[async_trait]
impl<TNext: Exchange> Exchange for NormalizedCacheImpl<TNext> {
    async fn run<Q: GraphQLQuery>(
        &self,
        operation: Operation<Q::Variables>
    ) -> ExchangeResult<Q::ResponseData> {
        if should_cache::<Q>(&operation) {
            if let Some(cached) = self.store.read_query::<Q>(&operation) {
                let response = OperationResult {
                    response: Response {
                        debug_info: Some(DebugInfo {
                            did_dedup: false,
                            source: ResultSource::Cache
                        }),
                        data: Some(cached),
                        errors: None
                    },
                    meta: operation.meta
                };
                Ok(response)
            } else {
                let variables: Q::Variables = operation.query.variables.clone();
                let res = self.next.run::<Q>(operation).await?;
                self.after_query::<Q>(&res, &variables)?;
                Ok(res)
            }
        } else {
            let operation_type = operation.meta.operation_type.clone();
            let variables = operation.query.variables.clone();
            let res = self.next.run::<Q>(operation).await?;
            if operation_type == OperationType::Mutation {
                self.after_mutation::<Q>(&res, variables);
            }
            Ok(res)
        }
    }
}
