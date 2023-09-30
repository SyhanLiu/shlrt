use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::Parser;
use syn::{parse_macro_input, DeriveInput};

// 类函数过程宏
#[proc_macro]
pub fn make_test(_item: TokenStream) -> TokenStream {
    "fn answer() -> u32 {40}".parse().unwrap()
}

// 派生宏
#[proc_macro_derive(AnswerFn)]
pub fn derive_answer_fn(_item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(_item as DeriveInput);
    let ident = input.ident;
    quote!(
        impl #ident {
            pub fn go_to_sleep(&self) {}
        }
    )
    .into()
}

// 属性宏
#[proc_macro_attribute]
pub fn attr_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());
    item
}
