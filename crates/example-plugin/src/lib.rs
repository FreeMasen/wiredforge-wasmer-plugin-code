// ./crates/example-plugin/src/lib.rs

/// This is the actual code we would 
/// write if this was a pure rust
/// interaction
pub fn length(s: &str) -> u32 {
    s.len() as u32
}

/// Since it isn't we need a way to
/// translate the data from wasm
/// to rust
#[no_mangle]
pub fn _length(ptr: i32, len: u32) -> u32 {
    // Extract the string from memory.
    let value = unsafe { 
        let slice = ::std::slice::from_raw_parts(ptr as _, len as _);
        String::from_utf8_lossy(slice)
    };
    //pass the value to `length` and return the result
    length(&value)
}