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
fn preprocess(_ctx: PreprocessorContext, mut book: Book) -> Result<Book, String> {
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