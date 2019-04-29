If you haven't seen it yet, you may want to checkout [part one](/blog/wasmer-plugin-pt-1/index.html) where we went over the basics of using wasmer. In this post we are going to cover how we could pass more complicated data from the WASM module back to the runner.

### Yet Another Plugin
To start we are going to create another plugin, this one will take a string as an argument and return that string doubled. Here is what that plugin would look like.


```rust
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
```

Most of what is going on here is exactly what we did the last time, the only difference is in that last line it has `.as_ptr()` added to it and the return value is now `i32`. `as_ptr` is a method that will return the byte index in memory of a value, which normally would be a pretty scary thing to deal with but I promise that we are going to survive. So how would we use this new plugin?

```rust
// ./crates/example-runner/src/main.rs
use wasmer_runtime::{
    imports,
    instantiate,
};

// For now we are going to use this to read in our WASM bytes
static WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/example_plugin.wasm");

fn main() {
    let instance = instantiate(&WASM, &imports!{}).expect("failed to instantiate WASM module");
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
    // loop over the WASM memory view's bytes
    // and also the string bytes
    for (cell, byte) in view[1..len + 1].iter().zip(bytes.iter()) {
        // set each WASM memory byte to 
        // be the value of the string byte
        cell.set(*byte)
    }
    // Bind our helper function
    let double = instance.func::<(i32, u32), i32>("_double").expect("Failed to bind _double");
    // Call the helper function an store the start of the returned string
    let start = double.call(1 as i32, len as u32).expect("Failed to execute _double") as usize;
    // Calculate the end as the start + twice the length
    let end = start + (len * 2);
    // Capture the string as bytes 
    // from a fresh view of the WASM memory
    let string_buffer: Vec<u8> = memory
                                    .view()[start..end]
                                    .iter()
                                    .map(|c|c.get())
                                    .collect();
    // Convert the bytes to a string
    let wasm_string = String::from_utf8(string_buffer)
                            .expect("Failed to convert WASM memory to string");
    println!("doubled: {}", wasm_string);
}
```

Again, almost all of this is going to be reused from the last example. We need to change the type arguments for `func` ever so slightly and the name of the function. Next we are going to call the `func` just like we did the last time, this time the return value is going to represent the index for the start of our new string. Since we will only ever double the string we can calculate the end by adding twice the original length plus the start, with both the start and the end we can capture the bytes as a slice. If you have the bytes as a slice you can try and convert it into a string using the `String::from_utf8` method. If we were to run this we should see the following.

```
cargo run
doubled: supercalifragilisticexpialidocioussupercalifragilisticexpialidocious
```

Huzzah! Success... though the situations where you would know the size of any data after a plugin ran is going to be too small to be useful. Now the big question becomes, if web assembly functions can only return 1 value how could we possibly know both the start and the length of any value coming back? One solution would be to reserve a section of memory that the WASM module could put the length in and then get the length when it's done.

## Two values from one function

Let's keep the same basic structure of our last plugin, this time though, we are going to get the length from a reserved part of memory.

```rust
pub fn double(s: &str) -> String {
    s.repeat(2)
}

#[no_mangle]
pub fn _double(ptr: i32, len: u32) -> i32 {
    // Extract the string from memory.
    let value = unsafe { 
        let slice = ::std::slice::from_raw_parts(ptr as _, len as _);
        String::from_utf8_lossy(slice)
    };
    // Double it
    let ret = double(&value);
    // Capture the length
    let len = ret.len() as u32;
    // write the length to byte 1 in memory
    unsafe {
        ::std::ptr::write(1 as _, len);
    }
    // return the start index
    ret.as_ptr() as _
}
```

This time in our plugin we have one change, the call to [`::std::ptr::write`](https://doc.rust-lang.org/std/ptr/fn.write.html), which will write to any place in memory you tell it to any value you want. This is a pretty dangerous thing to do, it is important that we have all our ducks in a row or we may corrupt some existing memory. This is going to write the 4 bytes that make up the variable `len` into memory at index 1, 2, 3, and 4. The key to making that work is that we are going to need to leave those 4 bytes empty when we insert our value from the runner.

Let's build that.

```
cargo -p example-plugin --target wasm32-unknown-unknown
```

Now we can get started on the runner.

```rust
// ./crates/example-runner/src/main.rs
use wasmer_runtime::{
    imports,
    instantiate,
};

// For now we are going to use this to read in our WASM bytes
static WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/example_plugin.wasm");

fn main() {
    let instance = instantiate(&WASM, &imports!{}).expect("failed to instantiate WASM module");
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
    // This is the string as bytes
    let bytes = s.as_bytes();
    // Our length of bytes
    let len = bytes.len();
    // loop over the WASM memory view's bytes
    // and also the string bytes
    for (cell, byte) in view[5..len + 5].iter().zip(bytes.iter()) {
        // set each WASM memory byte to 
        // be the value of the string byte
        cell.set(*byte)
    }
    // Bind our helper function
    let double = instance.func::<(i32, u32), i32>("_double").expect("Failed to bind _double");
    // Call the helper function an store the start of the returned string
    let start = double.call(5 as i32, len as u32).expect("Failed to execute _double") as usize;
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
    // from the new view of the WASM memory
    let string_buffer: Vec<u8> = new_view[start..end]
                                    .iter()
                                    .map(|c|c.get())
                                    .collect();
    // Convert the bytes to a string
    let wasm_string = String::from_utf8(string_buffer)
                            .expect("Failed to convert WASM memory to string");
    println!("doubled: {}", wasm_string);
}
```
Ok, a few more things are going on in this one. First we immediately update the memory's bytes 1 through 4 to be set to 0, this is where we are going to put the new length. We continue normally until after we call `_double`. This time through we are going to pull those first 4 bytes out of the WASM memory into a 4 byte array and convert that to a u32. We need to cast this u32 to a usize because we are going to be using it in as an index later. We can now update our `end` to use this new value instead of the old one. From that point on we keep going the same way. If we were to run this we should see the following.

```
cargo run
doubled: supercalifragilisticexpialidocioussupercalifragilisticexpialidocious
```


Huzzah! Success... and it is far more robust that before. If we executed a WASM module that exported `_double` that actually tripled a string or cut the string in half, we would still know the correct length. Now that we can pass arbitrary sets of bytes from rust to WASM and back again that means we have to tools to pass more complicated data. All we need now is a way to turn any struct into bytes and then back again, for that we can use something like [`bincode`](https://github.com/TyOverby/bincode) which is a binary serialization format used by [WebRender](https://github.com/servo/webrender) and [Servo's ipc-channel](https://github.com/servo/ipc-channel). It implements the traits defined by the [`serde`](https://serde.rs/) crate which greatly opens our options.

Since there are a bunch of `serde` trait implementations for a bunch of standard rust types including strings and tuples, let's leverage that to create a slightly more interesting example. 

### Slightly More Interesting™

First we want to update the dependencies for both our runner and plugin projects. Update the 2 Cargo.toml files to look like this.
```
# ./crates/example-runner/Cargo.toml
[package]
name = "example-runner"
version = "0.1.0"
authors = ["rfm <r@robertmasen.pizza>"]
edition = "2018"

[dependencies]
wasmer-runtime = "0.3.0"
bincode = "1"
```
```
# ./crates/example-plugin/Cargo.toml
[package]
name = "example-plugin"
version = "0.1.0"
authors = ["rfm <r@robertmasen.pizza>"]
edition = "2018"

[dependencies]
bincode = "1"

[lib]
crate-type = ["cdylib"]
```

Now we can use bincode both of these projects. This time around, the goal is going to be to create a plugin that will take a tuple of a u8 and a string and return an updated version of that tuple.

```rust
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
```

Just like last time time we take in our `ptr` and `len` arguments, we pass those along to `::std::slice::from_raw_parts` which creates a reference to our bytes. After we get those bytes we can deserialize them into a tuple of a u8 and a string. Now we can pass that tuple along to the `multiply` function and capture the results as `updated`. Next we are going to serialize that value into a `Vec<u8>` and as the variable `ret`. The rest is going to be exactly like our string example, capture the length, write it to memory index 1 and return the start index of the bytes. Let's build this.

```
cargo -p example-plugin --target wasm32-unknown-unknown
```

Now for our runner.

```rust
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

// For now we are going to use this to read in our WASM bytes
static WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/example_plugin.wasm");

fn main() {
    let instance = instantiate(&WASM, &imports!{}).expect("failed to instantiate WASM module");
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
    let pair = (u, s);
    let bytes = serialize(&pair).expect("Failed to serialize tuple");
    // Our length of bytes
    let len = bytes.len();
    // loop over the WASM memory view's bytes
    // and also the string bytes
    for (cell, byte) in view[5..len + 5].iter().zip(bytes.iter()) {
        // set each WASM memory byte to 
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
    // from the new view of the WASM memory
    let updated_bytes: Vec<u8> = new_view[start..end]
                                    .iter()
                                    .map(|c|c.get())
                                    .collect();
    // Convert the bytes to a string
    let updated: (u8, String) = deserialize(&updated_bytes)
                            .expect("Failed to convert WASM memory to tuple");
    println!("multiply {}: ({}, {:?})", pair.0, updated.0, updated.1);
}
```
First, we have updated our `use` statements to include some `std::time` items and the bincode functions for serializing and deserializing. We are going to use the same string as we did last time and calculate a pseudo random number between 1 and 10 that will serve as the parts of our tuple. Once we have constructed our tuple, we pass that off to `bincode::serialize` which gets us back to a `Vec<u8>`. We continue on just like our string example until after we get the new length back from the WASM module. At this point we are going to build the updated_bytes the same as before and pass those along to `bincode::deserialize` which should get us back to a tuple.

```
cargo run
multiply 2: (136, "supercalifragilisticexpialidocioussupercalifragilisticexpialidocious")
```

Huzzah! Another success! At this point it might be a good idea to address the ergonomics all of this, if we asked another developer to understand all of this, do you think anyone would build a plugin for our system? Probably not. In the next post we are going to cover how to ease that process by leveraging `proc_macros`.


[part three](/blog/wasmer-plugin-pt-3/index.html)
