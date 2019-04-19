#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std as std;
use bincode::{deserialize, serialize};
use example_macro::*;
#[doc = " This is the actual code we would "]
#[doc = " write if this was a pure rust"]
#[doc = " interaction"]
pub fn multiply(pair: (u8, String)) -> (u8, String) {
    let s = pair.1.repeat(pair.0 as usize);
    let u = pair.0.wrapping_mul(s.len() as u8);
    (u, s)
}
pub fn _multiply() {
    multiply((2, "attributed"));
}
