// ./crates/example-plugin/src/lib.rs

#[no_mangle]
pub fn add(one: i32, two: i32) -> i32 {
    one + two
}