use serde::Serialize;
use std::num::Wrapping;

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
