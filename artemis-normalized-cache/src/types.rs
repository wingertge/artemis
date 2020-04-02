use crate::QueryStore;
use artemis::{exchange::Extension, GraphQLQuery};
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::Arc
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};

#[derive(Default)]
pub struct NormalizedCacheOptions {
    pub custom_keys: Option<HashMap<&'static str, String>>
}

#[allow(clippy::type_complexity)]
#[derive(Default, Clone)]
pub struct NormalizedCacheExtension {
    pub(crate) optimistic_result:
        Option<Arc<dyn (Fn() -> Option<Box<dyn Any + Send>>) + Send + Sync>>,
    pub(crate) update:
        Option<Arc<dyn Fn(&(dyn Any + Send), QueryStore, &mut HashSet<String>) + Send + Sync>>,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update<Q: GraphQLQuery, F>(mut self, update: F) -> Self
    where
        F: Fn(&Option<Q::ResponseData>, QueryStore, &mut HashSet<String>) + Send + Sync + 'static
    {
        self.update = Some(Arc::new(move |data, store, dependencies| {
            let data = data.downcast_ref::<Option<Q::ResponseData>>().unwrap();
            update(data, store, dependencies);
        }));
        self
    }

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
