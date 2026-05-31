use crate::{derive_macro::expand_multipart, utils::expand_macro};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, ItemFn, LitStr, Path, Token, parse_macro_input, punctuated::Punctuated};
mod derive_macro;
mod utils;
macro_rules! macro_define {
    ( $($method:ident),* ) => {
        $(
            #[proc_macro_attribute]
            pub fn $method(_attr: TokenStream,input:TokenStream) -> TokenStream {
                let f = parse_macro_input!(input as ItemFn);
                let route = parse_macro_input!(_attr as LitStr);

                TokenStream::from(expand_macro(f,route,stringify!($method)))
            }
        )*
    };
}

macro_define!(get, post, delete, put);

/// Vec<HandlerModifier>
#[proc_macro]
pub fn handlers(input: TokenStream) -> TokenStream {
    // 解析为 Path 列表（支持 ident 和 a::b::c）
    let paths = parse_macro_input!(
        input with Punctuated::<Path, Token![,]>::parse_terminated
    );

    let expanded_items = paths.iter().map(|path| {
        quote! {
            Box::new(#path)
        }
    });

    quote! {
        vec![
            #(#expanded_items),*
        ]
    }
    .into()
}

#[proc_macro_derive(MultipartData, attributes(faithea))]
pub fn derive_multipart_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_multipart(&input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}
// #[proc_macro_attribute]
// pub fn get(_attr: TokenStream,input:TokenStream) -> TokenStream {
//     let f = parse_macro_input!(input as ItemFn);
//     let route = parse_macro_input!(_attr as LitStr);
//     TokenStream::from(expand_macro(f,route,"get"))
// }
