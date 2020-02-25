use serde::Serialize;
use std::num::Wrapping;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// When we have separate values it's useful to run a progressive
/// version of djb2 where we pretend that we're still looping over
/// the same value
pub fn progressive_hash<V: Serialize>(h: u64, x: &V) -> u64 {
    let x = bincode::serialize(x).expect("Failed to convert variables to Vec<u8> for hashing");

    let mut h = Wrapping(h);

    for i in 0..x.len() {
        h = (h << 5) + h + Wrapping(x[i] as u64)
    }

    h.0
}

#[macro_export]
macro_rules! ext {
    ($($x: expr),*) => {
        {
            let mut typemap = ::type_map::concurrent::TypeMap::new();
            $(
                typemap.insert($x)
            )*
            ::std::sync::Arc::new(typemap)
        }
    };
}

#[cfg(all(target_arch = "wasm32", feature = "observable"))]
pub mod wasm {
    use futures::{Stream, StreamExt};
    use js_sys::{Array, Function};
    use wasm_bindgen::{prelude::*, JsValue};

    #[wasm_bindgen(module = "wonka")]
    extern "C" {
        fn make(source_fn: &Closure<dyn FnOnce(&JsValue) -> &Closure<dyn FnOnce()>>) -> JsValue;
    }

    pub fn bind_stream<Stream, Item>(mut stream: Stream) -> JsValue
    where
        Stream: Stream<Item = Item>,
        Item: Into<JsValue>
    {
        let source_fn = |values: Array| {
            let this = JsValue::NULL;
            let next: &Function = values.get(0);
            let complete: &Function = values.get(0);
            let mut cancelled = false;

            while !cancelled {
                let next = stream.next().await;
                if let Some(next) = next {
                    next.call1(&this, next);
                } else {
                    complete.call0(&this);
                }
            }

            &Closure::new(|| {
                cancelled = true;
            })
        };
        let source_fn = Closure::new(source_fn);
        make(&source_fn)
    }
}
