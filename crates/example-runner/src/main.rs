use wasmer_runtime::{
    imports,
    instantiate,
};
// For now we are going to use this to read in our wasm bytes
static WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/example_plugin.wasm");

fn main() {
    // Instantiate the web assembly module
    let instance = instantiate(WASM, &imports!{}).expect("failed to instantiate wasm module");
    // Bind the add function from the module
    let add = instance.func::<(i32, i32), i32>("add").expect("failed to bind function add");
    // execute the add function
    let three = add.call(1, 2).expect("failed to execute add");
    println!("three: {}", three); // "three: 3"
}