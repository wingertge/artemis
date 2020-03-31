use std::num::Wrapping;

// TODO: Figure out why this gives different results on different OS
pub fn hash(x: &str) -> u32 {
    let x = x.as_bytes();
    let mut h = Wrapping(5381);
    for byte in x {
        h = (h << 5) + h + Wrapping(*byte as u32)
    }

    h.0
}
