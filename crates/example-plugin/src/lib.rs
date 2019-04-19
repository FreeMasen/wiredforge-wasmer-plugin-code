// ./crates/example-plugin/src/lib.rs
use bincode::{deserialize, serialize};
/// This is the actual code we would 
/// write if this was a pure rust
/// interaction
pub fn multiply(pair: (u8, String)) -> (u8, String) {
    // create a repeated version of the string
    // based on the u8 provided
    let s = pair.1.repeat(pair.0 as usize);
    // Multiply the u8 by the length
    // of the new string
    let u = pair.0.wrapping_mul(s.len() as u8);
    (u, s)
}

/// Since it isn't we need a way to
/// translate the data from wasm
/// to rust
#[no_mangle]
pub fn _multiply(ptr: i32, len: u32) -> i32 {
    // Extract the string from memory.
    let slice = unsafe { 
        ::std::slice::from_raw_parts(ptr as _, len as _)
    };
    // deserialize the memory slice
    let pair = deserialize(slice).expect("Failed to deserialize tuple");
    // Get the updated version
    let updated = multiply(pair);
    // serialize the updated value
    let ret = serialize(&updated).expect("Failed to serialize tuple");
    // Capture the length
    let len = ret.len() as u32;
    // write the length to byte 1 in memory
    unsafe {
        ::std::ptr::write(1 as _, len);
    }
    // return the start index
    ret.as_ptr() as _
}