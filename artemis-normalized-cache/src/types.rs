use artemis::GraphQLQuery;
use std::{any::Any, collections::HashMap};

#[derive(Default)]
pub struct NormalizedCacheOptions {
    pub custom_keys: Option<HashMap<&'static str, String>>
}

#[derive(Default)]
pub struct NormalizedCacheExtension {
    pub(crate) optimistic_result: Option<Box<dyn Fn() -> Option<Box<dyn Any + Send>>>>
}

impl NormalizedCacheExtension {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn optimistic_result<Q: GraphQLQuery, F: (Fn() -> Option<Q::ResponseData>) + 'static>(
        mut self,
        optimistic_result: F
    ) -> Self {
        self.optimistic_result = Some(Box::new(move || {
            optimistic_result().map(|res| -> Box<dyn Any + Send> { Box::new(res) })
        }));
        self
    }
}
