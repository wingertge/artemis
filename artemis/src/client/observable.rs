use crate::{client::ClientImpl, types::Observable, Exchange, GraphQLQuery, QueryError, QueryOptions, Response, progressive_hash};
use futures::{channel::mpsc::Sender, SinkExt};
use stable_vec::StableVec;
use std::{any::Any, future::Future, pin::Pin, sync::Arc};

pub type OperationObservable<Q, M> =
    Observable<Result<Response<<Q as GraphQLQuery>::ResponseData>, QueryError>, M>;

pub(crate) struct Subscription {
    pub(crate) listeners: StableVec<Sender<Arc<dyn Any + Send + Sync>>>,
    // This captures the type and variables of the query without requiring generics, so we can store it in a hashmap
    pub(crate) rerun: Arc<
        dyn Fn() -> Pin<Box<dyn Future<Output = Arc<dyn Any + Send + Sync>> + Send>> + Send + Sync
    >
}

pub async fn subscribe_with_options<Q: GraphQLQuery + 'static, M: Exchange>(
    client: &Arc<ClientImpl<M>>,
    _query: Q,
    variables: Q::Variables,
    options: QueryOptions
) -> super::observable::OperationObservable<Q, M> {
    let (query, meta) = Q::build_query(variables.clone());
    let (mut sender, receiver) = futures::channel::mpsc::channel(8);
    let key = progressive_hash(meta.query_key, &variables);

    let operation = client.create_request_operation::<Q>(query, meta, options.clone());

    let observable = {
        let mut subscriptions = client.active_subscriptions.lock();
        let index = if let Some(subscription) = subscriptions.get_mut(&key) {
            subscription.listeners.push(sender.clone())
        } else {
            let client = client.clone();
            let operation = operation.clone();
            let subscription = Subscription {
                listeners: vec![sender.clone()].into(),
                rerun: Arc::new(move || {
                    let client = client.clone();
                    let operation = operation.clone();

                    Box::pin(async move {
                        let res = client.execute_request_operation::<Q>(operation).await;
                        let res_boxed: Arc<dyn std::any::Any + Send + Sync> = Arc::new(res);
                        res_boxed
                    })
                })
            };
            subscriptions.insert(key.clone(), subscription);
            0
        };
        super::observable::Observable::new(key, receiver, client.clone(), index)
    };

    let res = client.execute_request_operation::<Q>(operation).await;
    sender.send(Arc::new(Box::new(res))).await.unwrap();
    observable
}

pub fn rerun_query<M: Exchange>(client: &Arc<ClientImpl<M>>, id: u64) {
    let client = client.clone();
    let fut = async move {
        let rerun = {
            let subscriptions = client.active_subscriptions.clone();
            let subscriptions = subscriptions.lock();
            subscriptions.get(&id).map(|sub| sub.rerun.clone())
        };
        let value = if let Some(rerun) = rerun {
            Some(rerun().await)
        } else {
            None
        };

        let subscriptions = client.active_subscriptions.clone();
        let mut subscriptions = subscriptions.lock();
        let subscription = subscriptions.get_mut(&id);

        if let Some(Subscription { listeners, .. }) = subscription {
            let value = value.unwrap();
            for listener in listeners.values_mut() {
                futures::executor::block_on(listener.send(value.clone())).unwrap();
            }
        }
    };
    spawn(fut);
}

#[cfg(target_arch = "wasm32")]
fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    wasm_bindgen_futures::spawn_local(fut);
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}
