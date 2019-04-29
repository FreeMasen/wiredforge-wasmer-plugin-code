+++
title = "Using Wasmer for Plugins Part 3"
date = 2019-04-22
draft = false
[extra]
snippet = "Now with more ease"
image = "rust-logo-blk.png"
date_sort = 20190422
image_desc = "Made by Freepik from www.flaticon.com, licensed by CC-3.0-BY"
+++

In the last two posts of this series we covered all of the things we would need to use [`Wasmer`](http://wasmer.io) as the base for a plugin system. In [part one](/blog/wasmer-plugin-pt-1/index.html) we went over the basics of passing simple data in and out of a web assembly module, in [part two](/blog/wasmer-plugin-pt-2/index.html) we dug deeper into how you might do the same with more complicated data. In this part we are going to explore how we might ease the experience for people developing plugins for our application. 

The majority of this is going to happen in a `proc_macro`, if you have never built one of these before, it can seem intimidating but we will go slow so don't fret. The first thing to understand is that `proc_macro`s are _meta-programming_, meaning we are writing code that writes code. Currently there are 3 options to chose from when writing a `proc_macro` but they all follow the same basic structure; a function that will take [`TokenStream`](https://doc.rust-lang.org/proc_macro/struct.TokenStream.html)s as arguments and return a `TokenStream`. A `TokenStream` is a collection of rust language parts, for example a keyword like `fn` or punctuation like `{`. It is almost like we are getting the text from a source file and returning a modified version of that text, though we get the added benefit of the fact that `rustc` is going to have validated it at least knows all of the parts in that text and will only let us add parts to it that it knows. To make this whole process a little easier, we are going to lean on a few crates pretty heavily, they are [`syn`](https://crates.io/crates/syn),  [`proc-macro2`](https://crates.io/crates/proc-macro2), and [`quote`](https://crates.io/crates/quote).
`syn` is going to parse the `TokenStream` into a structure that has more information, it will help answer questions like 'is this a function?' or 'is this function public?'. Many parts of that's structure are provided by `proc-macro2`. `quote` is going to help us create a `TokenStream` by "quasi-quoting" some rust text, we'll get into what that means in just a moment.

Now that we have our dependencies outlined, let's talk about the three types of `proc_macro`s. First we have a _custom derive_, if you have ever use the `#[derive(Serialize)]` attribute, you have used a custom derive. For these, we need to define a function that takes a single `TokenStream` argument and returns a new `TokenStream`, this return value will be append to the one passed in. That mean's we can't modify the original code, only augment it with something like an `impl` block, which makes it great for deriving a trait. Another option is often referred to as _function like_ macros, these look just like the macros created with `#[macro_rules]` when used but are defined using a similar system to the custom derives. The big difference between custom derives and function like macros is the return value for the latter is going to replace the argument provided, not extend it. Lastly we have the _attribute like_ macros, this is the one we are going to use. Attribute macros work the same as function like macros in that they will replace the code provided. The big difference is that an attribute definition the function we write will take 2 arguments, the first of which will be the contents of the attribute and the second is what that attribute is sitting on top of. To use the example from the rust book

```rust
#[route(GET, "/")]
fn index() {

}
```

The first argument is going to include `GET` and `"/"` and the second will contain the function `index`. With that basic structure defined, let's get started with our example. We are going to be making these edits in the `example-macro` project we added in part 1. Let's get those dependencies listed in the Cargo.toml.

```toml
# ./crates/example-macro/Cargo.toml
[package]
name = "example-macro"
version = "0.1.0"
authors = ["rfm <r@robertmasen.pizza>"]
edition = "2018"

[dependencies]
quote = "0.6"
proc-macro2 = "0.4"
syn = { version = "0.15", features = ["full"] }

[lib]
proc-macro = true

```

A few things to note here, first `syn` is pretty heavily feature gated, for this we want to add the "full" feature which will allow us to use all of the different types defined there. The next thing to point out is in the `[lib]` table we are going to add `proc-macro =  true` to tell cargo that this crate will only contain a proc_macro. Currently proc_macros need to be defined in their own crates. With that out of the way we can get started editing our `lib.rs`.

```rust
// ./crates/example-macro/src/lib.rs
extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn plugin_helper(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    tokens
}
```

First, we need to declare the use of the `proc_macro` crate that rust provides. Next we are going to use the `TokenStream` that is provided there. Our exported function is going to start with the `#[proc_macro_attribute]` attribute which will mark this function as an attribute with the same name. This function needs to take two arguments, both with the type `TokenStream` and return a `TokenStream`, just like we went over before. In this example we are just going to return the same value we were provided. Let's use our `example-plugin` project to see what it does. First we need to make sure that our macro is in the dependencies.

```toml
# ./crates/example-plugin/Cargo.toml
[package]
name = "example-plugin"
version = "0.1.0"
authors = ["rfm <r@robertmasen.pizza>"]
edition = "2018"

[dependencies]
bincode = "1"
example-macro = { path = "../example-macro" }

[lib]
crate-type = ["cdylib"]
```

Then we can use it like this.

```rust
// ./crates/example-plugin/src/lib.rs
use bincode::{deserialize, serialize};
use example_macro::*;
/// This is the actual code we would 
/// write if this was a pure rust
/// interaction
#[plugin_helper]
pub fn multiply(pair: (u8, String)) -> (u8, String) {
    // create a repeated version of the string
    // based on the u8 provided
    let s = pair.1.repeat(pair.0 as usize);
    // Multiply the u8 by the length
    // of the new string
    let u = pair.0.wrapping_mul(s.len() as u8);
    (u, s)
}
```

But... how can we see anything about this? We could `cargo build` to see if that works but that doesn't provide us much information. Thankfully there is a great 3rd party cargo command called `cargo-expand` that will help us out a ton. This utility relies on the nightly toolchain so we are going to need to get that first via rustup. To make things easier for later, let's also get the WASM target for the nightly toolchain.

```
rustup toolchain add nightly
rustup target add wasm32-unknown-unknown --target nightly
```

With that taken care of we can now install `cargo-expand`.

```
cargo install cargo-expand
```

If we were to run the following command it should print our expanded library to the console.

```
cd crates/example-plugin
cargo +nightly expand -p example-plugin
```
> As a side note, if you have an older version of cargo-expand installed it may not have the `-p` flag implemented, you can upgrade your version to current by running `cargo install --force cargo-expand` or simply run it from `crates/example-plugin`.

```rust
#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use ::std::prelude::v1::*;
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
```

This is the fully expanded output of our library, not much has change except that we can see a few things that rust will always do to our program like convert out doc comments to attributes. Now let's update our `proc_macro` to do something a little more interesting.

```rust
// ./crates/example-macro/src/lib.rs
extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{
   Item as SynItem,
};
use proc_macro2::{
   Ident,
   Span,
};
use quote::quote;

#[proc_macro_attribute]
pub fn plugin_helper(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    // convert the TokenStream into proc_macro2::TokenStream
    let tokens2 = proc_macro2::TokenStream::from(tokens);
    // parse the TokenStream into a syn::Item
    let parse2 = syn::parse2::<SynItem>(tokens2).expect("Failed to parse tokens");
    // Check if it is a function
    // if not panic
    match parse2 {
        SynItem::Fn(func) => handle_func(func),
        _ => panic!("Only functions are currently supported")
    }
}

fn handle_func(func: syn::ItemFn) -> TokenStream {
    // Copy the function's identifier
    let ident = func.ident.clone();
    // Create a new identifier with a underscore in front of 
    // the original identifier
    let shadows_ident = Ident::new(&format!("_{}", ident), Span::call_site());
    // Generate some rust with the original and new
    // shadowed function
    let ret = quote! {
        #func

        pub fn #shadows_ident() {
            #ident((2, String::from("attributed")));
        }
    };
    ret.into()
}
```
This time around we are first converting the `TokenStream` into the `proc_macro2::TokenStream` which will allow us to parse the tokens. The result of that is a `syn::Item` which is an enum of all the different types of rust `Item`s and will allow us to determine exactly what our attribute is decorating. For us, we only want this to work on functions, so we match `parse2`, if it is a `fn` we pass the inner data off to `handle_func` if not, we panic with a message about only supporting `fn`s.

Inside of `handle_func` we first make a copy of the original function's identifier, for our example that would be `multiply`. Next we are going to use that copy to create a new identifer that will have an underscore at the start: `_multiply`. To do this we are going to use the `proc_macro2::Ident` constructor which takes a `&str` and a `Span` (the index that this token takes up), we are going to use the `format!` macro for the first argument and thankfully `proc_macro2::Span` provides the `call_site` constructor that we can use which will figure out the index for us.  At this point we are going to use the `quote::quote` macro to generate a new `proc_macro2::TokenStream`. This is where that _quasi quoting_ happens, we can use the `#variable_name` syntax to insert variable's values into some raw text representing a rust program. First we want to put the original function as it was defined at the top, then we want to create a new function with our `_multiply` identifer the body of which will just call the original function with a constant set of arguments. Let's look at the expanded output. 

```
cargo expand -p example-plugin
```

```rust
#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use ::std::prelude::v1::*;
#[macro_use]
extern crate std as std;
// ./crates/example-plugin/src/lib.rs
use bincode::{deserialize, serialize};
use example_macro::*;

#[doc = " This is the actual code we would "]
#[doc = " write if this was a pure rust"]
#[doc = " interaction"]
pub fn multiply(pair: (u8, String)) -> (u8, String) {
    // create a repeated version of the string
    // based on the u8 provided
    let s = pair.1.repeat(pair.0 as usize);
    // Multiply the u8 by the length
    // of the new string
    let u = pair.0.wrapping_mul(s.len() as u8);
    (u, s)
}
pub fn _multiply() { multiply((2, String::from("attributed"))); }

```

Another relatively useless transformation but we did successfully generate some code with our macro, now let's get back to our actual goal. If we look back at part 2's last helper function our end goal is going to replicated the following.

```rust
#[no_mangle]
pub fn _multiply(ptr: i32, len: u32) -> i32 {
    let slice = unsafe { 
        ::std::slice::from_raw_parts(ptr as _, len as _)
    };
    let pair = deserialize(slice).expect("Failed to deserialize tuple");
    let updated = multiply(pair);
    let ret = serialize(&updated).expect("Failed to serialize tuple");
    let len = ret.len() as u32;
    unsafe {
        ::std::ptr::write(1 as _, len);
    }
    ret.as_ptr() as _
}
```

We should be able to reproduce that function with our attribute if we just extend the last example a little.

```rust
// ./crates/example-macro/src/lib.rs
#![recursion_limit="128"]
extern crate proc_macro;
use proc_macro::TokenStream;

use syn::{Item as SynItem, ItemFn};
use quote::quote;
use proc_macro2::{Ident, Span};

#[proc_macro_attribute]
pub fn plugin_helper(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let tokens2 = proc_macro2::TokenStream::from(tokens);
    let parse2 = syn::parse2::<SynItem>(tokens2).expect("Failed to parse tokens");
    match parse2 {
        SynItem::Fn(func) => handle_func(func),
        _ => panic!("Only functions are currently supported")
    }
}

fn handle_func(func: ItemFn) -> TokenStream {
    // Check and make sure our function takes
    // only one argument and panic if not
    if func.decl.inputs.len() != 1 {
        panic!("fns marked with plugin_helper can only take 1 argument");
    }
    // Copy this function's identifier
    let ident = func.ident.clone();
    // Create a new identifier with a underscore in front of 
    // the original identifier
    let shadows_ident = Ident::new(&format!("_{}", ident), Span::call_site());
    // Generate some code with the original and new
    // shadowed function
    let ret = quote! {
        #func

        #[no_mangle]
        pub fn #shadows_ident(ptr: i32, len: u32) -> i32 {
            let value = unsafe {
                ::std::slice::from_raw_parts(ptr as _, len as _)
            };
            let arg = deserialize(value).expect("Failed to deserialize argument");
            let ret = #ident(arg);
            let bytes = serialize(&ret).expect("Failed to serialize return value");
            let len = bytes.len();
            unsafe {
                ::std::ptr::write(1 as _, len);
            }
            bytes.as_ptr()
        }
    };
    ret.into()
}
```

You may notice at the top we need to add the module attribute `#![recursion_limit="128"]`, this is because `quote` does some seriously deep recursion to work its magics. The next change is to add a check that there is only one argument and panic if not to simplify our plugins. We use the same scheme for generating a new identifier for our new function and then we really just ripped to code right out of the last example, replacing `multiply(pair)` with `#ident(arg)`. If we run cargo expand on that we get the following.

```rust
#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use ::std::prelude::v1::*;
#[macro_use]
extern crate std as std;
// ./crates/example-plugin/src/lib.rs
use bincode::{deserialize, serialize};
use example_macro::*;
#[doc = " This is the actual code we would "]
#[doc = " write if this was a pure rust"]
#[doc = " interaction"]
pub fn multiply(pair: (u8, String)) -> (u8, String) {
    // create a repeated version of the string
    // based on the u8 provided
    let s = pair.1.repeat(pair.0 as usize);
    // Multiply the u8 by the length
    // of the new string
    let u = pair.0.wrapping_mul(s.len() as u8);
    (u, s)
}
#[no_mangle]
pub fn _multiply(ptr: i32, len: u32) -> i32 {
    let value = unsafe { ::std::slice::from_raw_parts(ptr as _, len as _) };
    let arg = deserialize(value).expect("Failed to deserialize argument");
    let ret = multiply(arg);
    let bytes = serialize(&ret).expect("Failed to serialize return value");
    let len = bytes.len() as u32;
    unsafe { ::std::ptr::write(1 as _, len); }
    bytes.as_ptr() as _
}
```
Looks a lot like our last example from part 2!

Let's try and compile that to WASM and execute the runner.

```
cargo build -p example-plugin --target wasm32-unknown-unknown
cargo run -p example-runner
multiply 10: (72, "supercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocious")
```

Huzzah! It still works! We are still requiring that plugin developers know a little too much about the inner workings of our system though. Let's use the library we put in the workspace root to take care of this last little hurdle. Instead of importing the macro directly into the plugin, if we were to import it into our library, we would have a more convenient package to provide to plugin developers. We can also take care of our dependencies problem at the same time. Let's update that project to package all of our requirements for the plugin developer, starting with the dependencies.

```toml
# ./Cargo.toml
[package]
name = "wasmer-plugin-example"
version = "0.1.0"
authors = ["rfm <r@robertmasen.pizza>"]
edition = "2018"

[dependencies]
serde = "1"
bincode = "1"
example-macro = { path = "./crates/example-macro" }

[workspace]
members = [
    "./crates/example-macro",
    "./crates/example-plugin",
    "./crates/example-runner",
]
```

Now in that library we can use the `pub use` keywords to re-export our macro and also define a couple of helper functions.

```rust
// ./src/lib.rs
use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize};

pub use example_macro::plugin_helper;

pub fn convert_data<'a, D>(bytes: &'a [u8]) -> D 
where D: Deserialize<'a> {
    deserialize(bytes).expect("Failed to deserialize bytes")
}

pub fn revert_data<S>(s: S) -> Vec<u8> 
where S: Serialize {
    serialize(s).expect("Failed to serialize data")
}
```

We are essentially wrapping the bincode functions we are using in identical function. It would probably be smarter to have these return results but for now this will do. The big win here is that our users will only need to import our library and not need to worry about having `serde` and `bincode` available. With those defined we can make a small update in the `example-macro` to use them. 

```rust
// ./crates/example-macro/src/lib.rs
#![recursion_limit="128"]
extern crate proc_macro;
use proc_macro::TokenStream;

use syn::{Item as SynItem, ItemFn};
use quote::quote;
use proc_macro2::{Ident, Span};

#[proc_macro_attribute]
pub fn plugin_helper(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let tokens2 = proc_macro2::TokenStream::from(tokens);
    let parse2 = syn::parse2::<SynItem>(tokens2).expect("Failed to parse tokens");
    match parse2 {
        SynItem::Fn(func) => handle_func(func),
        _ => panic!("Only functions are currently supported")
    }
}

fn handle_func(func: ItemFn) -> TokenStream {
    // Check and make sure our function takes
    // only one argument and panic if not
    if func.decl.inputs.len() != 1 {
        panic!("fns marked with plugin_helper can only take 1 argument");
    }
    // Copy this function's identifier
    let ident = func.ident.clone();
    // Create a new identifier with a underscore in front of 
    // the original identifier
    let shadows_ident = Ident::new(&format!("_{}", ident), Span::call_site());
    // Generate some rust with the original and new
    // shadowed function
    let ret = quote! {
        #func

        #[no_mangle]
        pub fn #shadows_ident(ptr: i32, len: u32) -> i32 {
            let value = unsafe {
                ::std::slice::from_raw_parts(ptr as _, len as _)
            };
            let arg = convert_data(value);
            let ret = #ident(arg);
            let bytes = revert_data(&ret);
            let len = bytes.len() as u32;
            unsafe {
                ::std::ptr::write(1 as _, len);
            }
            bytes.as_ptr() as _
        }
    };
    ret.into()
}
```

Now we need to point our plugin to the workspace root instead of the macro directly which means we can get rid of the bincode dependency.

```toml
# ./crates/example-plugin/Cargo.toml
[package]
name = "example-plugin"
version = "0.1.0"
authors = ["rfm <r@robertmasen.pizza>"]
edition = "2018"

[dependencies]
wasmer-plugin-example = { path = "../.." }

[lib]
crate-type = ["cdylib"]
```
With that updated we can now adjust the use statement to `use wasmer_plugin_example::*`
```rust
// ./crates/example-plugin/src/lib.rs
use wasmer_plugin_example::*;
/// This is the actual code we would 
/// write if this was a pure rust
/// interaction
#[plugin_helper]
pub fn multiply(pair: (u8, String)) -> (u8, String) {
    // create a repeated version of the string
    // based on the u8 provided
    let s = pair.1.repeat(pair.0 as usize);
    // Multiply the u8 by the length
    // of the new string
    let u = pair.0.wrapping_mul(s.len() as u8);
    (u, s)
}
```
Let's just double check that we haven't broken anything.

```
cargo build -p example-plugin --target wasm32-unknown-unknown
cargo run -p example-runner
multiply 5: (82, "supercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocious")
```

Huzzah! It works and that looks a lot cleaner than before, now plugin developers don't need to worry about how we are doing what we do but instead can just focus on their task. In the next part, we are going to cover a real world example of how you might use this scheme to extend an application. We are going to focus on extending [mdbook](https://github.com/rust-lang-nursery/mdBook) to allow web assembly plugins for preprocessing.