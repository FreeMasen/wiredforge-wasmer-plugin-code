In the last three posts of this series we covered all of the things we would need to use [`Wasmer`](http://wasmer.io) as the base for a plugin system. In [part one](/blog/wasmer-plugin-pt-1/index.html) we went over the basics of passing simple data in and out of a web assembly module, in [part two](/blog/wasmer-plugin-pt-2/index.html) we dug deeper into how you might do the same with more complicated data. In the [last part](/blog/wasmer-plugin-pt-3/index.html) we eased the experience of plugin developers by encapsulating all of our work into a library that exports a procedural macro. In this post we are going to explore what it would take to extend an existing plugin system to allow for WASM plugins. 

## Enter MDBook

Before we get started with any code, we should first go over [mdbook](https://github.com/rust-lang-nursery/mdBook) a little bit. If you are not familiar, mdbook is an application that enables its users to create books using markdown files and a toml file for configuration. You are probably familiar with the format because [TRPL](https://doc.rust-lang.org/book/index.html) is built using it and while HTML is probably the most popular output it has the ability to render into a few other formats. These other formats are provided through a plugin system which has two sides, preprocessors and renderers. Each side is really aptly named, the preprocessors will get the information first then the renderer will get the information last. Both types of plugins communicate with the main mdbook process via stdin and stdout. The basic workflow is that mdbook will read in the book and it's contents from the file system, generate a struct that represents that book and then serializes it to json and pipes it to a child process. If that child process is a preprocessor, it will deserialize, update, re-serialize and then pipe that back, if it is a render it will deserialize and then render that however it likes. At this point, we are going to focus on the preprocessor because WASM isn't currently a great candidate for dealing with the file system or network and the preprocessor doesn't need any of that. 

In the [official guide](https://rust-lang-nursery.github.io/mdBook/for_developers/preprocessors.html) the mdbook team outlined the basic structure as being an struct that implements the trait `Preprocessor` which requires two methods `name`, `run` and allows an optional method `supports` which by default returns true. The main entry point being the `run` method, which take a `PreprocessorContext` and a `Book` and returns a `Result<Book>`. While this is a good way to explain what is needed, in actuality a preprocessor would look a little different. First, instead of a struct that implements a trait, it can just be a command line application that can support running with no arguments as well as with the `supports` argument. If the supports argument is provided, the application should use the exit status code to indicate if it does (0) or does not (1) support a particular renderer. If no argument was provided we would then deserialize the context and book provided from stdin (as a tuple). Once those two values are acquired, you can manipulate the book however you'd like and then serialize the it and send that back out via stdout. Let's quickly look at what a preprocessor might look like if it just updates any "WASM" strings to "Wasm" (because WASM isn't an acronym). For this example, we are going to update the runner. First we want to add a few more dependencies, namely mdBook, docopt, serde and serde_derive.

``` toml
# ./crates/example-runner/Cargo.toml
[package]
name = "mdbook-example-runner"
version = "0.1.0"
authors = ["rfm <r@robertmasen.pizza>"]
edition = "2018"

[dependencies]
wasmer-runtime = "0.3.0"
bincode = "1"
mdbook = { git = "https://github.com/rust-lang-nursery/mdBook" }
docopt = "1"
serde = "1"
serde_derive = "1"
serde_json = "1"
```
Two things to point out here, first is that we are updating the name of this program to have a prefix of `mdbook-` this is a requirement of any mdbook preprocessor, the other is that we are using mdbook as a git dependency. As of the writing of this post there is an issue with their handlebars dependency that would make the library fail to compile to WASM. The next version of mdbook will not include this problem but for now, this example will need to work with the git repository instead of crates.io. We are going to use docopt for command line argument parsing but you could just as easily use clap, structopt or DIY it if you'd prefer. 

As a note, this example is going to remove a lot of the wasmer-runtime stuff for readability (you may want to keep some of it around for later if you're typing along).

```rust
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
    book::{
        Book,
        BookItem,
    },
    preprocess::PreprocessorContext,
};

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
    let (_ctx, book): (PreprocessorContext, Book) = 
        from_reader(stdin())
        .expect("Failed to deserialize context and book");
    // Update the book's contents
    let updated = preprocess(book)
        .expect("Failed to preprocess book");
    // serialize and write the updated book
    // to stdout
    to_writer(stdout(), &updated)
        .expect("Failed to serialize/write book");
}

/// Update the book's contents so that all WASMs are
/// replaced with Wasm
fn preprocess(mut book: Book) -> Result<Book, String> {
    // Iterate over the book's sections assigning
    // the updated items to the book we were passed
    book.sections = book.sections.into_iter().map(|s| {
        // each section could be a chapter
        // or a seperator
        match s {
            // if its a chapter, we want to update that
            BookItem::Chapter(mut ch) => {
                // replace all WASMs with Wasms
                ch.content = ch.content.replace("WASM", "Wasm");
                // Wrap the contents back up into a Chapter
                BookItem::Chapter(ch)
            },
            _ => s,
        }
    }).collect();
    // Return the updated book
    Ok(book)
}
```

If you have never used docopt, it essentially uses command line usage text as a serialization format. To start we are going to define our usage. With that done we can declare the struct that will represent the deserialized command line arguments. Docopt uses a prefix scheme for flags vs sub-commands vs arguments, we want to have a field `arg_supports` that will be an optional string. Now we can actually get into the execution, first we pass the usage off to docopt and exit early if it fails to parse. Next we want to check if the caller provided the supports argument, if so we are just going to exit early with 0 which just says yes, we support this format. Once we are through that we can use the serde_json function deserialize_from to both read stdin and also serialize it into a tuple with a context first and the book second. Now that we have those two items we are going to pass them along to the function preprocess. 

For this preprocessor, we are going loop over all of the sections in the book and any chapters we find and update the contents of those to replace any "WASM"s with "Wasm"s returning the updated book. We are going to use the serde_json function `serialize_to` to serialize the returned book to json and write that to stdout. As you can see, this is both a powerful system but also one that requires plugin developers to know quite a bit about how everything works. After building a [preprocessor](https://github.com/FreeMasen/mdbook-presentation-preprocessor) myself and then hearing about wasmer-runtime it seemed like a perfect opportunity to make this whole thing easier.

If we wanted to test our first example out we would need mdbook installed and an actual book to run it against. To install mdbook, [you have a few options](https://github.com/FreeMasen/wasmer-plugin) but for this example we will use `cargo install mdbook`. With that installed we can create a book with the following.

```
mdbook init ./example-book

Do you want a .gitignore to be created? (y/n)
n
What title would you like to give the book? 
Example Book
```

As an example, the [repo](https://github.com/FreeMasen/wiredforge-wasmer-plugin-code) has one defined with the contents of this series, with all the wasms capitalized. Now, we need to tell mdbook to run our preprocessor, we do that in the `book.toml` file.

```toml
# ./example-book/book.toml
[book]
authors = ["rfm"]
multilingual = false
src = "src"
title = "Example Book"

[preprocessor.example-runner]
```

We are almost there, the last thing we need to do is install our plugin, we do that with the following command.

```
cargo install --path ./crates/example-runner
```

Cargo will compile that for us and put it in our path. We can now run `mdbook build ./example-book`, which will generate a bunch of files in the `./example-book/book` directory, any of the html files should have their WASMs updated to Wasms.

One of the really nice things about there being an existing plugin system is that we don't need to be maintainers to realize our vision. We could define our own scheme for running WASM plugins that interfaces with mdbook via the old system. Let's say that we want our plugin developers to provide a functions `preprocess(mut book: Book) -> Book`. Since this takes a single argument and return a single argument, we can use the same scheme to execute it as we have previously. Let's take the WASM to WASM part from above and move that into our example plugin, to do that we need to update the dependencies.

```toml
# ./crates/example-plugin/Cargo.toml
[package]
name = "example-plugin"
version = "0.1.0"
authors = ["rfm <r@robertmasen.pizza>"]
edition = "2018"

[dependencies]
wasmer-plugin-example = { path = "../.." }

[dependencies.mdbook]
git = "https://github.com/rust-lang-nursery/mdBook"
default-features = false 

[lib]
crate-type = ["cdylib"]
```

Adding a dependency with a toml table like we're doing for mdbook is a nice way to make it clearer what is happening. Again we are going to point to the git repository, we also need to make sure that the default-features are turned off. The mdbook default features are primarily for the binary application, avoiding them is the other key to allowing this to compile to WASM. With that out of the way we can update our code.

```rust
// ./crates/example-plugin/src/lib.rs
use wasmer_plugin_example::*;
use mdbook::{
    book::{
        Book,
        BookItem,
    },
    preprocess::PreprocessorContext,
};
#[plugin_helper]
pub fn preprocess(mut book: Book) -> Book {
    // Iterate over the book's sections assigning
    // the updated items to the book we were passed
    book.sections = book.sections.into_iter().map(|s| {
        // each section could be a chapter
        // or a seperator
        match s {
            // if its a chapter, we want to update that
            BookItem::Chapter(mut ch) => {
                // replace all WASMs with Wasms
                ch.content = ch.content.replace("WASM", "Wasm");
                // Wrap the contents back up into a Chapter
                BookItem::Chapter(ch)
            },
            _ => s,
        }
    }).collect();
    // Return the updated book
    book
}
```

Here we have updated the library to export a function called `preprocess` annotated with the `#[plugin_helper]` attribute which means we should be able to use it just like we did before. Now we can update our runner, we are going to be passing what we have deserialized from the command line to the WASM module.

```rust
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

// For now we are going to use this to read in our WASM bytes
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
    let (_ctx, book): (PreprocessorContext, Book) = 
        from_reader(stdin())
        .expect("Failed to deserialize context and book");
    // Update the book's contents
    let updated = preprocess(book)
        .expect("Failed to preprocess book");
    // serialize and write the updated book
    // to stdout
    to_writer(stdout(), &updated)
        .expect("Failed to serialize/write book");
}

/// Update the book's contents so that all WASMs are
/// replaced with Wasm
fn preprocess(book: Book) -> Result<Book, String> {
    let instance = instantiate(&WASM, &imports!{})
        .expect("failed to instantiate WASM module");
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
    let bytes = serialize(&book)
        .expect("Failed to serialize tuple");
    // Our length of bytes
    let len = bytes.len();
    // loop over the WASM memory view's bytes
    // and also the string bytes
    for (cell, byte) in view[5..len + 5]
                .iter()
                .zip(bytes.iter()) {
        // set each WASM memory byte to 
        // be the value of the string byte
        cell.set(*byte)
    }
    // Bind our helper function
    let wasm_preprocess = instance.func::<(i32, u32), i32>("_preprocess")
        .expect("Failed to bind _preprocess");
    // Call the helper function an store the start of the returned string
    let start = wasm_preprocess.call(5 as i32, len as u32)
        .expect("Failed to execute _preprocess") as usize;
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
    // from the new view of the WASM memory
    let updated_bytes: Vec<u8> = new_view[start..end]
                                    .iter()
                                    .map(|c|c.get())
                                    .collect();
    // Convert the bytes to a string
    deserialize(&updated_bytes)
        .map_err(|e| format!("Error deserializing after WASM update\n{}", e))
}
```

A lot of what we see in `preprocess` should look familiar to our previous runner examples, the only real change being that `pair` will now just be `book` and the name of the function we are calling has changed. At this point, to test if this is working we would need to rebuild the plugin and then re-install the runner before we can build our book.

```
cargo build -p example-plugin
cargo install --path ./crates/example-runner --force
mdbook build example-book
```
When we run that the output in example-book/book should now have to content we expect. One last thing to cover is that we are still using the `include_bytes` macro to get our WASM. If this was a real plugin system we would need a method for getting that in a more dynamic way. Let's assume that we want our users to put any pre-compiled WASM preprocessors into a new sub-directory of the book's root called preprocessors. For this example we can just move our last example plugin into this new folder.

```
mkdir ./example-book/preprocessors
cp ./target/wasm32-unknown-unknown/debug/example-plugin.WASM ./example-book/preprocessors
```

Now we can update our runner to look in that directory instead of compiling the bytes into the binary file.

```rust
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
        Read,
    },
    fs::File,
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
    if let Some(renderer_name) = opts.arg_supports {
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
    let updated = run_all_preprocessors(ctx, book)
        .expect("Failed to preprocess book");
    // serialize and write the updated book
    // to stdout
    to_writer(stdout(), &updated)
        .expect("Failed to serialize/write book");
}

fn run_all_preprocessors(ctx: PreprocessorContext, mut book: Book) -> Result<Book, String> {
    // ctx.root will tell us where our book lives
    let dir = ctx.root.join("preprocessors");
    // loop over all of the preprocessors files there
    for entry in dir.read_dir()
        .map_err(|e| format!("Error reading preprocessors directory {}", e))? {
        // safely unwrap the dir entry
        let entry = entry
            .map_err(|e| format!("Error reading entry {}", e))?;
        // pull out the path we are working on
        let path = entry.path();
        // Check if the path ends with .wasm
        if let Some(ext) = path.extension() {
            if ext == "wasm" {
                // if it does we want to read all the bytes into
                // a buffer
                let mut buf = Vec::new();
                let mut f = File::open(&path)
                    .map_err(|e| format!("Error opening file {:?}, {}", path, e))?;
                f.read_to_end(&mut buf)
                    .map_err(|e| format!("Error reading file {:?}, {}", path, e))?;
                // We can now pass this off to our original preprocess
                book = preprocess(buf.as_slice(), book)?;
            }
        }
    }
    Ok(book)
}

/// Update the book's contents so that all WASMs are
/// replaced with Wasm
fn preprocess(bytes: &[u8], book: Book) -> Result<Book, String> {
    // instantiate the WASM module with the bytes provided
    let instance = instantiate(bytes, &imports!{})
        .expect("failed to instantiate WASM module");
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
    let bytes = serialize(&book)
        .expect("Failed to serialize tuple");
    // Our length of bytes
    let len = bytes.len();
    // loop over the WASM memory view's bytes
    // and also the string bytes
    for (cell, byte) in view[5..len + 5]
                .iter()
                .zip(bytes.iter()) {
        // set each WASM memory byte to 
        // be the value of the string byte
        cell.set(*byte)
    }
    // Bind our helper function
    let wasm_preprocess = instance.func::<(i32, u32), i32>("_preprocess")
        .expect("Failed to bind _preprocess");
    // Call the helper function an store the start of the returned string
    let start = wasm_preprocess.call(5 as i32, len as u32)
        .expect("Failed to execute _preprocess") as usize;
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
    // from the new view of the WASM memory
    let updated_bytes: Vec<u8> = new_view[start..end]
                                    .iter()
                                    .map(|c|c.get())
                                    .collect();
    // Convert the bytes to a string
    deserialize(&updated_bytes)
        .map_err(|e| format!("Error deserializing after WASM update\n{}", e))
}
```

The big changes here is that we are passing the context and book off to `run_all_preprocessors` instead of just `preprocess`. In this new function we are going to first construct the path that will contain our WASM preprocessors. The context will have a `root` field that will tell us where our book lives, we can append "preprocessors" on to that with `join`. Now that we have our path we want to loop over each of the files in that directory and if then end in .WASM we want to pass those bytes off to `preprocess` with the book. The result of that should replace our previous book and we will return the updated book after all of the WASM files have been run. All in all we seem to have a pretty viable plugin runner. There may be a few places that could use some tweaks to increase resiliency or reduce the memory footprint but at least it should be enough to get started.

If your interested I have built a less educational version of this plugin system which you can find [here](https://github.com/FreeMasen/wasmer-plugin). My hope is that I can add a few more niceties in the coming months that will focus on the plugin runner side of things (like extracting more data from errors in WASM or wrapping up the instantiate/serialize/inject/execute/extract/deserialize cycle). If you have any comments, questions, suggestions or gripes feel free to shoot me an email at r [at] robertmasen.com or find me on twitter @freemasen.
