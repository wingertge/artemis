use std::num::Wrapping;

pub fn hash(x: &&str) -> u64 {
    let x = x.as_bytes();
    let mut h = Wrapping(5381u64);
    for i in 0..x.len() {
        h = (h << 5) + h + Wrapping(x[i] as u64)
    }

    h.0
}
