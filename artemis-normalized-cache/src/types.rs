use crate::QueryStore;
use artemis::{exchange::Extension, GraphQLQuery};
use std::{any::Any, collections::HashMap, sync::Arc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};

/// Options to pass to the normalized cache.
#[derive(Default)]
pub struct NormalizedCacheOptions {
    /// An optional `HashMap` of typenames to unique ID keys.
    /// The keys are the names of the fields, not the IDs themselves.
    /// So if your `User` has a unique ID called `ident`, you should
    /// set `"User" => "ident"`.
    /// The default ID keys are `id` and `_id`, so those don't need to be mapped.
    pub custom_keys: Option<HashMap<&'static str, String>>
}

/// A query extension that lets you pass additional logic into the cache.
#[allow(clippy::type_complexity)]
#[derive(Default, Clone)]
pub struct NormalizedCacheExtension {
    pub(crate) optimistic_result:
        Option<Arc<dyn (Fn() -> Option<Box<dyn Any + Send>>) + Send + Sync>>,
    pub(crate) update:
        Option<Arc<dyn Fn(&(dyn Any + Send), QueryStore, &mut Vec<String>) + Send + Sync>>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) update_js: Option<js_sys::Function>
}

impl Extension for NormalizedCacheExtension {
    #[cfg(target_arch = "wasm32")]
    fn from_js(value: JsValue) -> Option<Self> {
        let update: JsValue = "update".into();
        let update = js_sys::Reflect::get(&value, &update).ok();
        let optimistic: JsValue = "optimistic".into();
        let optimistic = js_sys::Reflect::get(&value, &optimistic).ok();

        let mut result = NormalizedCacheExtension::new();

        if let Some(update) = update {
            let update: js_sys::Function = update.dyn_into().unwrap();
            result = NormalizedCacheExtension {
                update_js: Some(update),
                ..result
            }
        }

        if let Some(optimistic) = optimistic {
            let optimistic: js_sys::Function = optimistic.dyn_into().unwrap();
            let optimistic = artemis::wasm::JsFunction(optimistic);
            result = result.optimistic_result(move || {
                let this = JsValue::NULL;
                let result = optimistic.0.call0(&this);
                serde_wasm_bindgen::from_value::<Option<serde_json::Value>>(result).unwrap()
            });
        }

        Some(result)
    }
}

impl NormalizedCacheExtension {
    /// Create a new query extension with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// A custom updater function to run against related queries, such as lists of the same entity
    /// The function has 3 parameters:
    ///
    /// * `current_data` - The returned data of the query you're running. This may also be an
    /// optimistic result.
    /// * `store` - A [`QueryStore`](./struct.QueryStore.html) object used to run custom update
    /// logic against other queries.
    /// * `dependencies` - This must be passed through to the `QueryStore` without modification.
    ///
    /// # Example
    ///
    /// ```
    /// # use artemis_normalized_cache::NormalizedCacheExtension;
    /// # use artemis_test::queries::get_conferences::{GetConferences, get_conferences::{self, Variables, ResponseData}};
    /// # use artemis_test::get_conference::{GetConference, get_conference};
    ///
    /// let extension = NormalizedCacheExtension::new()
    ///     .update::<GetConference, _>(|current_data, store, dependencies| {
    ///         let conference = current_data.as_ref().unwrap().conference.as_ref().unwrap();
    ///         store.update_query(GetConferences, Variables, move |mut data| {
    ///             if let Some(ResponseData { conferences: Some(ref mut conferences) }) = data {
    ///                 conferences.push(get_conferences::GetConferencesConferences {
    ///                     id: conference.id.clone(),
    ///                     name: conference.name.clone()
    ///                 });
    ///             }
    ///             data
    ///         }, dependencies)
    ///     });
    /// ```
    pub fn update<Q: GraphQLQuery, F>(mut self, update: F) -> Self
    where
        F: Fn(&Option<Q::ResponseData>, QueryStore, &mut Vec<String>) + Send + Sync + 'static
    {
        self.update = Some(Arc::new(move |data, store, dependencies| {
            let data = data.downcast_ref::<Option<Q::ResponseData>>().unwrap();
            update(data, store, dependencies);
        }));
        self
    }

    /// A function returning an optimistic result. This result will be written to a temporary cache
    /// layer and will be pushed to subscribers immediately. If subscribers aren't used the
    /// optimistic result is ignored.
    ///
    /// This closure must return an optional `ResponseData` - `None` is interpreted as
    /// `skip optimistic result`.
    pub fn optimistic_result<
        Q: GraphQLQuery,
        F: (Fn() -> Option<Q::ResponseData>) + 'static + Send + Sync
    >(
        mut self,
        optimistic_result: F
    ) -> Self {
        self.optimistic_result = Some(Arc::new(move || {
            optimistic_result().map(|res| -> Box<dyn Any + Send> { Box::new(res) })
        }));
        self
    }
}
