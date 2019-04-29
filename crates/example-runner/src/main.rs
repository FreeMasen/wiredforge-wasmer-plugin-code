// ./crates/example-runner/src/main.rs
use docopt::Docopt;
use serde::Deserialize;
use serde_json::{
    from_reader, 
    to_writer,
};
use std::{
    process::exit,
    io::{
        stdin,
        stdout,
    }
};
use mdbook::{
    book::Book,
    preprocess::PreprocessorContext,
};
use bincode::{
    serialize,
    deserialize,
};
use wasmer_runtime::{
    instantiate,
    imports,
};

// For now we are going to use this to read in our wasm bytes
static WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/example_plugin.wasm");

static USAGE: &str = "
Usage:
    mdbook-wasm-preprocessor
    mdbook-wasm-preprocessor supports <supports>
";

#[derive(Deserialize)]
struct Opts {
    pub arg_supports: Option<String>,
}

fn main() {
    // Parse and deserialize command line
    // arguments
    let opts: Opts = Docopt::new(USAGE)
                    .and_then(|d| d.deserialize())
                    .unwrap_or_else(|e| e.exit());
    // If the arg supports was include
    // we need to handle that
    if let Some(_renderer_name) = opts.arg_supports {
        // This will always resolve
        // to `true` for mdbook
        exit(0);
    }
    // Parse and deserialize the context and book
    // from stdin
    let (ctx, book): (PreprocessorContext, Book) = 
        from_reader(stdin())
        .expect("Failed to deserialize context and book");
    // Update the book's contents
    let updated = preprocess(ctx, book)
        .expect("Failed to preprocess book");
    // serialize and write the updated book
    // to stdout
    to_writer(stdout(), &updated)
        .expect("Failed to serialize/write book");
}

/// Update the book's contents so that all WASMs are
/// replaced with Wasm
fn preprocess(ctx: PreprocessorContext, book: Book) -> Result<Book, String> {
    println!("creating instance");
    let instance = instantiate(&WASM, &imports!{}).expect("failed to instantiate wasm module");
    // The changes start here
    // First we get the module's context
    println!("capturing context");
    let context = instance.context();
    // Then we get memory 0 from that context
    // web assembly only supports one memory right
    // now so this will always be 0.
    println!("capturing memory");
    let memory = context.memory(0);
    // Now we can get a view of that memory
    println!("getting first view");
    let view = memory.view::<u8>();
    // Zero our the first 4 bytes of memory
    println!("zeroing cell 1-4");
    for cell in view[1..5].iter() {
        cell.set(0);
    }
    let pair = (ctx, book);
    println!("serializing pair");
    let bytes = serialize(&pair)
        .expect("Failed to serialize tuple");
    // Our length of bytes
    let len = bytes.len();
    // loop over the wasm memory view's bytes
    // and also the string bytes
    println!("injecting bytes");
    for (cell, byte) in view[5..len + 5].iter().zip(bytes.iter()) {
        // set each wasm memory byte to 
        // be the value of the string byte
        cell.set(*byte)
    }
    println!("binding preprocessor");
    // Bind our helper function
    let wasm_preprocess = instance.func::<(i32, u32), i32>("_preprocess")
        .expect("Failed to bind _multiply");
    println!("running preprocessor");
    // Call the helper function an store the start of the returned string
    let start = wasm_preprocess.call(5 as i32, len as u32)
        .expect("Failed to execute _multiply") as usize;
    // Get an updated view of memory
    let new_view = memory.view::<u8>();
    // Setup the 4 bytes that will be converted
    // into our new length
    let mut new_len_bytes = [0u8;4];
    for i in 0..4 {
        // attempt to get i+1 from the memory view (1,2,3,4)
        // If we can, return the value it contains, otherwise
        // default back to 0
        new_len_bytes[i] = new_view
            .get(i + 1)
            .map(|c| c.get())
            .unwrap_or(0);
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
    deserialize(&updated_bytes)
        .map_err(|e| format!("Error deserializing after wasm update\n{}", e))
}