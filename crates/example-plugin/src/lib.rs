// ./crates/example-plugin/src/lib.rs

/// This is the actual code we would 
/// write if this was a pure rust
/// interaction
pub fn double(s: &str) -> String {
    s.repeat(2)
}

/// Since it isn't we need a way to
/// translate the data from wasm
/// to rust
#[no_mangle]
pub fn _double(ptr: i32, len: u32) -> i32 {
    // Extract the string from memory.
    let value = unsafe { 
        let slice = ::std::slice::from_raw_parts(ptr as _, len as _);
        String::from_utf8_lossy(slice)
    };
    // pass the value to `double` and 
    // return the result as a pointer
    double(&value).as_ptr() as i32
}