// ./crates/example-plugin/src/lib.rs
use wasmer_plugin_example::*;
/// This is the actual code we would 
/// write if this was a pure rust
/// interaction
#[plugin_helper]
pub fn multiply(pair: (u8, String)) -> (u8, String) {
    // create a repeated version of the string
    // based on the u8 provided
    let s = pair.1.repeat(pair.0 as usize);
    // Multiply the u8 by the length
    // of the new string
    let u = pair.0.wrapping_mul(s.len() as u8);
    (u, s)
}