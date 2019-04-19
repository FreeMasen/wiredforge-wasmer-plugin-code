// ./crates/example-macro/src/lib.rs
extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn plugin_helper(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let tokens2 = proc_macro2::TokenStream::from(tokens);
    let parse2 = syn::parse2::<SynItem>(tokens2).expect("Failed to parse tokens");
    match parse2 {
        SynItem::Fn(func) => handle_func(func),
        _ => panic!("Only functions are currently supported")
    }
}

fn handle_func(func: syn::ItemFn) -> TokenStream {
    // Copy this function's identifier
    let ident = func.ident.clone();
    // Create a new identifier with a underscore in front of 
    // the original identifier
    let shadows_ident = Ident::new(&format!("_{}", ident), Span::call_site());
    // Generate some rust with the original and new
    // shadowed function
    let ret = quote! {
        #func

        pub fn #shadows_ident() {
            #ident((2, "attributed"));
        }
    };
    ret.into()
}