// ./crates/example-plugin/src/lib.rs
use example_macro::*;

#[plugin_helper]
pub fn attributed() {
    println!("attributed!")
}