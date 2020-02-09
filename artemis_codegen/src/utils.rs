use std::num::Wrapping;

pub fn hash(x: &&str) -> u32 {
    let x = x.as_bytes();
    let mut h = Wrapping(5381u32);
    for i in 0..x.len() {
        h = (h << 5) + h + Wrapping(x[i] as u32)
    }

    h.0
}
