// ./crates/example-runner/src/main.rs
use wasmer_runtime::{
    imports,
    instantiate,
};

use std::time::{
    UNIX_EPOCH,
    SystemTime,
};

use bincode::{
    deserialize,
    serialize,
};

// For now we are going to use this to read in our wasm bytes
static WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/example_plugin.wasm");

fn main() {
    let instance = instantiate(&WASM, &imports!{}).expect("failed to instantiate wasm module");
    // The changes start here
    // First we get the module's context
    let context = instance.context();
    // Then we get memory 0 from that context
    // web assembly only supports one memory right
    // now so this will always be 0.
    let memory = context.memory(0);
    // Now we can get a view of that memory
    let view = memory.view::<u8>();
    // Zero our the first 4 bytes of memory
    for cell in view[1..5].iter() {
        cell.set(0);
    }
    // This is the string we are going to pass into wasm
    let s = "supercalifragilisticexpialidocious".to_string();
    let now = SystemTime::now();
    let diff = now.duration_since(UNIX_EPOCH).expect("Failed to calculate timestamp");
    let u = ((diff.as_millis() % 10) + 1) as u8;
    let pair = (u , s);
    let bytes = serialize(&pair).expect("Failed to serialize tuple");
    // Our length of bytes
    let len = bytes.len();
    // loop over the wasm memory view's bytes
    // and also the string bytes
    for (cell, byte) in view[5..len + 5].iter().zip(bytes.iter()) {
        // set each wasm memory byte to 
        // be the value of the string byte
        cell.set(*byte)
    }
    // Bind our helper function
    let double = instance.func::<(i32, u32), i32>("_multiply").expect("Failed to bind _multiply");
    // Call the helper function an store the start of the returned string
    let start = double.call(5 as i32, len as u32).expect("Failed to execute _multiply") as usize;
    // Get an updated view of memory
    let new_view = memory.view::<u8>();
    // Setup the 4 bytes that will be converted
    // into our new length
    let mut new_len_bytes = [0u8;4];
    for i in 0..4 {
        // attempt to get i+1 from the memory view (1,2,3,4)
        // If we can, return the value it contains, otherwise
        // default back to 0
        new_len_bytes[i] = new_view.get(i + 1).map(|c| c.get()).unwrap_or(0);
    }
    // Convert the 4 bytes into a u32 and cast to usize
    let new_len = u32::from_ne_bytes(new_len_bytes) as usize;
    // Calculate the end as the start + new length
    let end = start + new_len;
    // Capture the string as bytes 
    // from the new view of the wasm memory
    let updated_bytes: Vec<u8> = new_view[start..end]
                                    .iter()
                                    .map(|c|c.get())
                                    .collect();
    // Convert the bytes to a string
    let updated: (u8, String) = deserialize(&updated_bytes)
                            .expect("Failed to convert wasm memory to tuple");
    println!("multiply {}: ({}, {:?})", pair.0, updated.0, updated.1);
}