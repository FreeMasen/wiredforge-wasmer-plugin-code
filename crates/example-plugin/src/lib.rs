// ./crates/example-plugin/src/lib.rs
use wasmer_plugin_example::*;
use mdbook::{
    book::{
        Book,
        BookItem,
    },
};

#[cfg_attr(target_arch = "wasm32", plugin_helper)]
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

#[cfg(test)]
mod test {
    use super::*;
    use mdbook::book::BookBuilder;
    #[test]
    fn check() {
        let b = BookBuilder::new("../../example-book").build().unwrap();
        let updated = preprocess(b.book);
        for section in updated.sections {
            match section { 
                mdbook::book::BookItem::Chapter(ch) => {
                    assert!(ch.content.find("WASM").is_none());
                },
                _ => (),
            }
        }
    }
    #[test]
    fn ser() {
        let b = BookBuilder::new("../../example-book").build().unwrap();
        let de = revert_data(b.book);
        let s = convert_data(de.as_slice());
        let updated = preprocess(s);
        for section in updated.sections {
            match section { 
                mdbook::book::BookItem::Chapter(ch) => {
                    assert!(ch.content.find("WASM").is_none());
                },
                _ => (),
            }
        }
    }
}