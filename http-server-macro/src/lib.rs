use proc_macro::TokenStream;
use quote::quote;
use syn::{ Ident,  ItemFn, LitStr, parse_macro_input, punctuated::Punctuated,Token};
use crate::utils::expand_macro ;
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

macro_define!(get,post,delete,put);


#[proc_macro]
pub fn handlers(input: TokenStream) -> TokenStream {
    // 解析输入为 ident 列表： a, b, c
    let idents = parse_macro_input!(input with Punctuated::<Ident, Token![,]>::parse_terminated);

    // 为每个 ident 生成 ident_abc
    let expanded_items = idents.iter().map(|ident| {
        let new_ident = syn::Ident::new(&format!("{}", ident), ident.span());
        quote! {
            Box::new(#new_ident)
        }
    });

    // 生成最终 vec![...]
    let output = quote! {
        vec![
            #(#expanded_items),*
        ]
    };

    output.into()
}


// #[proc_macro_attribute]
// pub fn get(_attr: TokenStream,input:TokenStream) -> TokenStream {
//     let f = parse_macro_input!(input as ItemFn);
//     let route = parse_macro_input!(_attr as LitStr);
//     TokenStream::from(expand_macro(f,route,"get"))
// }
