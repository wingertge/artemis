pub mod add_conference;
pub mod get_conference;
pub mod get_conferences;
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use super::{add_conference::*, get_conference::*, get_conferences::*};
    use artemis::{
        client::ClientImpl,
        exchange::Exchange,
        wasm::{JsQueryError, QueryCollection},
        GraphQLQuery, QueryOptions
    };
    use std::sync::Arc;
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    #[derive(Copy, Clone, PartialEq)]
    #[repr(u32)]
    pub enum Queries {
        GetConference = 1354603040u32,
        GetConferences = 2299573645u32,
        AddConference = 1424555866u32
    }
    impl QueryCollection for Queries {
        fn query<M: Exchange>(
            self,
            client: Arc<ClientImpl<M>>,
            variables: JsValue,
            options: QueryOptions
        ) -> ::futures::future::BoxFuture<'static, Result<JsValue, JsValue>> {
            let fut = Box::pin(async move {
                match self {
                    Queries::GetConference => {
                        let variables = serde_wasm_bindgen::from_value::<
                            <GetConference as GraphQLQuery>::Variables
                        >(variables)
                        .unwrap();
                        let response = client
                            .query_with_options(GetConference, variables, options)
                            .await;
                        response
                            .map(|response| serde_wasm_bindgen::to_value(&response).unwrap())
                            .map_err(|e| {
                                serde_wasm_bindgen::to_value(&JsQueryError::from(e)).unwrap()
                            })
                    }
                    Queries::GetConferences => {
                        let variables = serde_wasm_bindgen::from_value::<
                            <GetConferences as GraphQLQuery>::Variables
                        >(variables)
                        .unwrap();
                        let response = client
                            .query_with_options(GetConferences, variables, options)
                            .await;
                        response
                            .map(|response| serde_wasm_bindgen::to_value(&response).unwrap())
                            .map_err(|e| {
                                serde_wasm_bindgen::to_value(&JsQueryError::from(e)).unwrap()
                            })
                    }
                    Queries::AddConference => {
                        let variables = serde_wasm_bindgen::from_value::<
                            <AddConference as GraphQLQuery>::Variables
                        >(variables)
                        .unwrap();
                        let response = client
                            .query_with_options(AddConference, variables, options)
                            .await;
                        response
                            .map(|response| serde_wasm_bindgen::to_value(&response).unwrap())
                            .map_err(|e| {
                                serde_wasm_bindgen::to_value(&JsQueryError::from(e)).unwrap()
                            })
                    }
                }
            });
            Box::pin(::artemis::wasm::UnsafeSendFuture::new(fut))
        }
        fn subscribe<M: Exchange>(
            self,
            client: Arc<ClientImpl<M>>,
            variables: JsValue,
            callback: js_sys::Function,
            options: QueryOptions
        ) {
            match self {
                Queries::GetConference => {
                    let variables = serde_wasm_bindgen::from_value::<
                        <GetConference as GraphQLQuery>::Variables
                    >(variables)
                    .unwrap();
                    let observable =
                        client.subscribe_with_options(GetConference, variables, options);
                    ::artemis::wasm::bind_stream(observable, callback);
                }
                Queries::GetConferences => {
                    let variables = serde_wasm_bindgen::from_value::<
                        <GetConferences as GraphQLQuery>::Variables
                    >(variables)
                    .unwrap();
                    let observable =
                        client.subscribe_with_options(GetConferences, variables, options);
                    ::artemis::wasm::bind_stream(observable, callback);
                }
                Queries::AddConference => {
                    let variables = serde_wasm_bindgen::from_value::<
                        <AddConference as GraphQLQuery>::Variables
                    >(variables)
                    .unwrap();
                    let observable =
                        client.subscribe_with_options(AddConference, variables, options);
                    ::artemis::wasm::bind_stream(observable, callback);
                }
            }
        }
    }
}
