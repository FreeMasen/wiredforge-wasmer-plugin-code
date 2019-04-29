# Chapter 1


A few months ago, the [Wasmer](https://wasmer.io) team announced a Web Assembly (aka wasm) interpreter that could be embedded into rust programs. This is particularly exciting for anyone looking to add plugins to their project and since Rust provides a way to directly compile programs to wasm, it seems like a perfect option. In this series of blog posts we are going to investigate what building a plugin system using wasmer and rust would take. 

## The Setup

Before we really dig into the specifics, we should have a layout in mind for our project. That way if you want to follow along on your own computer, you can and if your not, nothing will seem like _magic_. To do this we are going to take advantage of cargo's workspace feature which allows us to collect a bunch of related projects in one parent project. You can also find a github repo with all of the code [here](https://github.com/FreeMasen/wiredforge-wasmer-plugin-code), each branch will represent a different state of this series. The basic structure we are going to shoot for would look something like this.
```
wasmer-plugin-example
├── Cargo.toml
├── crates
│   ├── example-macro
│   │   ├── Cargo.toml
│   │   └── src
│   │       └── lib.rs
│   ├── example-plugin
│   │   ├── Cargo.toml
│   │   └── src
│   │       └── lib.rs
│   └── example-runner
│       ├── Cargo.toml
│       └── src
│           └── main.rs
└── src
    └── lib.rs
```
- `wasmer-plugin-example` - A rust library, the details of which we will cover in detail in one of the next parts
  - `crates` - The folder that will house all of our other projects
    - `example-plugin` - The plugin we will use to test that everything is working as expected
    - `example-runner` - A Binary project that will act as our plugin host
    - `example-macro` - A `proc_macro` library that we will be creating in one of the next parts

To set this up we are going to start by creating the parent project.

```
cargo new --lib wasmer-plugin-example
cd wasmer-plugin-example
```
Once that has been created we can move into that directory and in your editor of choice you would then open the Cargo.toml. We need to add a `[workspace]` table to the configuration and point to the 3 projects in the `crates` folder from above.

```toml
[package]
name = "wasmer-plugin-example"
version = "0.1.0"
authors = ["freemasen <r@wiredforge.com>"]
edition = "2018"

[dependencies]


[workspace]
members = [
    "./crates/example-macro",
    "./crates/example-plugin",
    "./crates/example-runner",
]
```

Now we can make that `crates` folder and the projects that will live inside it.

```
mkdir ./crates
cd ./crates
cargo new --lib example-plugin
cargo new --lib example-macro
cargo new example-runner
```

With that we have our workspace setup. This will allow us to use cargo commands from any of the directories inside our project and target activity in any other project in our workspace. We tell cargo which project we want an action to apply to with the `-p` argument. If we wanted to build the `example-plugin` project for instance we would use the following command.

```
cargo build -p example-plugin
```

With our workspace all setup, we should take a moment and get our development environment in order. First and for most we need to have the rust compiler, `cargo` and `rustup`. If you need those head over to [rustup.rs](https://rustup.rs/). With all that installed we are going to need the web assembly target from `rustup`.  

```
rustup target add wasm32-unknown-unknown
```

In addition to are rust requirements, we will also need a few things for wasmer. The full guide is available [here](https://github.com/wasmerio/wasmer#dependencies), for most system you just need to make sure `cmake` is installed, for windows it is slightly more complicated but there are links on dependency guide.

## Our First  Plugin
With that out of the way, we should talk about the elephant in the room, the Web Assembly specification only allows for the existence of numbers. Thankfully the web assembly target for rust can already handle this inside of a single program for us but any function in a plugin we want to call from our runner will need to only take numbers as arguments and only return numbers. With that in mind let's start with a very simple example. I will note that the examples in this part will not be very useful but I promise we will slowly build up the ability to do much more interesting things. 

```rust
// ./crates/example-plugin/src/lib.rs
#[no_mangle]
pub fn add(one: i32, two: i32) -> i32 {
    one + two
}
```

The above is an extremely naive and uninteresting example of what a plugin might look like but it fits our requirement that it only deals with numbers. Now to get this to compile to Web Assembly, we need to set one more thing up in our `Cargo.toml`.

```toml
# ./crates/example-plugin/Cargo.toml
[package]
name = "example-plugin"
version = "0.1.0"
authors = ["freemasen <r@wiredforge.com>"]
edition = "2018"

[dependencies]


[lib]
crate-type = ["cdylib"]
```

The key here is the `crate-type = ["cdylib"]`, which says that we want this crate to be compiled as a C dynamic library. Now we can compile it with the following command

```
cargo build --target wasm32-unknown-unknown
```

At this point we should have a file in `./target/wasm32-unknown-unknown/debug/example_plugin.wasm`. Now that we have that, let's build a program that will run this, first we will get our dependencies all setup.

## Our First Runner
```toml
# ./crates/example-runner/Cargo.toml
[package]
name = "example-runner"
version = "0.1.0"
authors = ["freemasen <r@wiredforge.com>"]
edition = "2018"

[dependencies]
wasmer_runtime = "0.3.0"
```
Here we are adding the `wamer_runtime` crate which we will use to interact with our web assembly module.

```rust
// ./crates/example-runner/src/main.rs
use wasmer_runtime::{
    imports,
    instantiate,
};
// For now we are going to use this to read in our WASM bytes
static WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/example_plugin.wasm");

fn main() {
    // Instantiate the web assembly module
    let instance = instantiate(WASM, &imports!{}).expect("failed to instantiate WASM module");
    // Bind the add function from the module
    let add = instance.func::<(i32, i32), i32>("add").expect("failed to bind function add");
    // execute the add function
    let three = add.call(1, 2).expect("failed to execute add");
    println!("three: {}", three); // "three: 3"
}
``` 
First, we have our `use` statement, there was are just grabbing 2 things; the `imports` macro for easily defining our import object and the `instantiate` function for converting bytes into a web assembly module instance. We are going to use the `include_bytes!` macro for now to read our bytes but eventually we will want to make this a little more flexible.  Inside of our `main` we are going to call `instantiate` with the WASM bytes as the first argument and an empty imports object as the second. Next we are going to use the `func` method on `instance` to bind the function `add` giving it the arguments types of two `i32`s and a return value of an `i32`. At this point we can use the `call` method on the function `add`, and then print the result to the terminal. When we `cargo run` it should successfully print `three: 3` in the terminal.

Huzzah, success! but that isn't super useful. Let's investigate what we would need to make it more useful.

## Digging Deeper
### Our requirements
1. Access to the WASM Memory before our function runs
2. A way to insert a more complicated data structure into that memory
3. A method to communicate where and what the data is to the WASM module
4. A system for extracting the update information from the WASM memory after the plugin is executed

First we need a way to initialize some value into the WASM module's memory before we run our function. Thankfully `wasmer_runtime` gives us a way to do exactly that. Let's update our example to take in a string and return the length of that string, this isn't going to be much more useful than our last example but... baby steps.

![Bill Murray everyone...](https://media0.giphy.com/media/NAe117ka9jAdi/giphy.gif?cid=790b76115cb8b4c2565a54784d25a2f4)

### Our Second Plugin
```rust
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
```

There is quite a bit more that we needed to do this time around, let's go over what is happening. First we have defined a function `length`, this is exactly what we would want to if we were using this library from another rust program. Since we are using this library as a WASM module, we need to add a helper that will deal with all of the memory interactions. This may seem like an odd structure but doing it this way allows for additional flexibility which will become more clear as we move forward. The `_length` function is going to be that helper. First, we need the arguments and return values to match what is available when crossing the WASM boundary (only numbers). Our arguments then will describe the shape of our string, `ptr` is the start of the string and `len` is the length. Since we are dealing with raw memory, we need to do the conversion inside of an `unsafe` block (I know that is a bit scary but we are going to make sure that there actually is a string there in the runner). Once we pull the string out of memory, we can pass it over to `length` just like normal, returning the result. Go ahead and build it just like before.

```
cargo build --target wasm32-unknown-unknown
```

Now let's cover how we would set this up in the runner.

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
    let length = instance.func::<(i32, u32), u32>("_length").expect("Failed to bind _length");
    let wasm_len = match length.call(1 as i32, len as u32) {
        Ok(l) => l,
        Err(e) => panic!("{}\n\n{:?}", e, e),
    }; //.expect("Failed to execute _length");
    println!("original: {}, wasm: {}", len, wasm_len); // original: 34, wasm: 34
}
```

Ok, there is quite a bit more going on this time around. The first few lines are going to be exactly the same, we are going to read in the WASM and then instantiate it. Once that is done, we are going to get a view into the WASM memory, we do this by first getting the `Ctx` (context) from the module instance. Once we have the context we can pull out the memory by calling `memory(0)`, web assembly only has one memory currently so in the short term this will always take the value 0 but moving forward there may be more than one memory allowed. One last step to actually get the raw memory is to call the `view()` method, we are finally at a stage where we can modify the module's memory. The type of `view` is `Vec<Cell<u8>>`, so we have a vector of bytes but each of the bytes is wrapped in a `Cell`. A [`Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html) according to the documentation is a way to allow mutating one part of an immutable value, in our case it is essentially saying "I'm not going to make this memory any longer or shorter, just change what its values are". 

Now we define the string we want to pass into the WASM memory and convert that to bytes. We also want to keep track of the byte length of that string so we capture that as `len`. To put the string bytes into the memory bytes we are going to use the [`Zip`](https://doc.rust-lang.org/std/iter/struct.Zip.html) iterator, which just lets us loop over two things at one time. In each iteration of our loop, we are going to stop at both the cell and the string byte in the same index, in the loop body we are setting the value of the WASM memory byte to the value of the string's byte. Notice that we started at index 1 in the `view`, that means our `ptr` parameter is going to be 1 and our byte length is going to be the `len` parameter.

```
cargo run
original: 34, wasm: 34
```

Huzzah! Success again! But alas, still pretty useless. It does however give us a good foundation to build upon for working with more complicated data. We saw how to interact with the WASM memory on both sides of the equation which we will exploit in part 2.


[part two](/blog/wasmer-plugin-pt-2/index.html)
