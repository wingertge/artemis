mod queries;
pub use queries::*;
use std::sync::{Arc, Mutex};

pub(crate) type Long = String;

pub type SyncCounter = Arc<Mutex<Counter>>;

#[derive(Debug)]
pub struct Counter {
    n: u32
}

impl Counter {
    pub fn new() -> Self {
        Counter { n: 0 }
    }

    pub fn sync() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self { n: 0 }))
    }

    pub fn inc(&mut self) {
        self.n += 1;
    }

    pub fn inc_sync(counter: &Arc<Mutex<Self>>) {
        let mut this = counter.lock().unwrap();
        this.n += 1;
    }

    pub fn get_sync(counter: &Arc<Mutex<Self>>) -> u32 {
        counter.lock().unwrap().n.clone()
    }
}

impl PartialEq<u32> for Counter {
    fn eq(&self, other: &u32) -> bool {
        &self.n == other
    }
}
