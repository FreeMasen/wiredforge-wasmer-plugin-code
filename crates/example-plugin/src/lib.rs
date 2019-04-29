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
pub fn preprocess((_ctx, mut book): (PreprocessorContext, Book)) -> Book {
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