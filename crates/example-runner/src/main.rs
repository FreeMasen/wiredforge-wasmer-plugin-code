// ./crates/example-runner/src/main.rs
use wasmer_runtime::{
    imports,
    instantiate,
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
    // This is the string we are going to pass into wasm
    let s = "supercalifragilisticexpialidocious".to_string();
    // This is the string as bytes
    let bytes = s.as_bytes();
    // Our length of bytes
    let len = bytes.len();
    // loop over the wasm memory view's bytes
    // and also the string bytes
    for (cell, byte) in view[1..len + 1].iter().zip(bytes.iter()) {
        // set each wasm memory byte to 
        // be the value of the string byte
        cell.set(*byte)
    }
    // Bind our helper function
    let double = instance.func::<(i32, u32), i32>("_double").expect("Failed to bind _length");
    // Call the helper function an store the start of the returned string
    let start = double.call(1 as i32, len as u32).expect("Failed to execute _length") as usize;
    // Calculate the end as the start + twice the length
    let end = start + (len * 2);
    // Capture the string as bytes 
    // from a fresh view of the wasm memory
    let string_buffer: Vec<u8> = memory
                                    .view()[start..end]
                                    .iter()
                                    .map(|c|c.get())
                                    .collect();
    // Convert the bytes to a string
    let wasm_string = String::from_utf8(string_buffer)
                            .expect("Failed to convert wasm memory to string");
    println!("doubled: {}", wasm_string);
}