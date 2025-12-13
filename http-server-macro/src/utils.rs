use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, Ident, ItemFn, LitStr, Pat, PatIdent,  Type, TypePath, parse_quote, punctuated::Punctuated, token::Comma};

pub fn add_return_type(f: &mut ItemFn) {
    f.sig.output = syn::parse_quote! {
        -> impl http_server::response::HttpResponseModifier
    }
}
pub enum FromHttpRequest {
    PathParam(LitStr),
    _Shared(LitStr),
    Body,
}
impl FromHttpRequest {
    pub fn into_token_stream(self) -> TokenStream {
        use FromHttpRequest::*;
        match self {
            PathParam(name) => {
                quote! {
                    _req.get_pathparam(#name).ok_or("err".to_string())?.convert()?,
                }
            }
            Body => {
                quote! {
                    (&_req).try_into()?,
                }
            }
            _Shared(name) => {
                quote! {
                    _req.get_shared(#name).ok_or("err".to_string())?.clone(),
                }
            }
        }
    }
}

pub fn generate_from_httprequest_list(
    args: &Punctuated<FnArg, Comma>
) -> Vec<FromHttpRequest> {
    args.iter().map(|arg| parse_arg(arg)).collect()
}

fn parse_arg(arg: &FnArg) -> FromHttpRequest {
    let (name, ty) = extract_ident_and_type(arg)
        .expect("Only typed `ident: Type` arguments are supported");

    let outer = outer_type_name(ty);

    match outer.as_deref() {
        Some("Json")   => FromHttpRequest::Body,
        Some("Shared") => FromHttpRequest::_Shared(name),
        _              => FromHttpRequest::PathParam(name),
    }
}

/// 提取参数名 (LitStr) 和 类型 (Type)
fn extract_ident_and_type(arg: &FnArg) -> Option<(LitStr, &Type)> {
    if let FnArg::Typed(t) = arg {
        if let Pat::Ident(PatIdent { ident, .. }) = t.pat.as_ref() {
            let name = LitStr::new(&ident.to_string(), ident.span());
            return Some((name, t.ty.as_ref()));
        }
    }
    None
}

/// 获取类型外层名字：Json<T> → Some("Json")
fn outer_type_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            path.segments.last().map(|seg| seg.ident.to_string())
        }
        _ => None,
    }
}
// pub fn generate_from_httprequest_list(ipt: &Punctuated<FnArg, Comma>) -> Vec<FromHttpRequest> {
//     ipt.iter()
//         .map(|i| {
//             if let FnArg::Typed(t) = i {
//                 if let Pat::Ident(arg_name) = t.pat.as_ref() {
//                     let arg_name =
//                         LitStr::new(arg_name.ident.to_string().as_str(), Span::call_site());
//                     let ty = t.ty.as_ref();
//                     if let Type::Path(p) = ty {
//                         let p = &p.path.segments;
//                         let  seg:Vec<&Ident> = p.iter().map(|x| &x.ident).collect();
//                         if let Some(s) = seg.last() {
//                             match s.to_string().as_str() {
//                                 "Json" => {
//                                     return FromHttpRequest::Body;
//                                 }
//                                 "Shared" => return FromHttpRequest::_Shared(arg_name),
//                                 _ => {}
//                             }
//                         }
//                         return FromHttpRequest::PathParam(arg_name);
//                     }
//                 }
//             }
//             unreachable!()
//         })
//         .collect::<Vec<FromHttpRequest>>()
// }

fn create_handler_fn_name(sig: &str) -> Ident {
    let new_fn_name = format!("{}_handler", sig);
    Ident::new(&new_fn_name, Span::call_site())
}
fn conbine_outter_fn(f: &ItemFn, args: Vec<FromHttpRequest>, orign_name: &str) -> TokenStream {
    let new_fn_name = create_handler_fn_name(orign_name);
    let ipt_args = args.into_iter().map(FromHttpRequest::into_token_stream);
    let inner_handler_name = &f.sig.ident;
    quote! {
        async fn #new_fn_name(_req: http_server::request::HttpRequest) -> Result<http_server::response::HttpResponse, String> {
            use  http_server::request::ConvertFromRefString;
            use  http_server::response::HttpResponseModifier;

            #f

            let mut res = http_server::response::HttpResponse::new();

            let res_modifier = #inner_handler_name(
                #(#ipt_args)*
                _req
            )
            .await;

            res_modifier.modify(&mut res)?;
            Ok(res)
        }
    }
}
fn add_req_param(f:&mut ItemFn) {
    f.sig.inputs.push(parse_quote!(_req:http_server::request::HttpRequest));
}
pub fn handler_fn(f: &mut ItemFn, name: &str) -> TokenStream {
    let ipt_args = generate_from_httprequest_list(&f.sig.inputs);
    add_req_param(f);
    conbine_outter_fn(f, ipt_args, name)
}

pub fn handler_modifier_fn(
    handler_fn: TokenStream,
    route: LitStr,
    method: &str,
    origin_name: &str,
) -> TokenStream {
    let handler_modifier_fn_name = Ident::new(origin_name, Span::call_site());
    let handler_fn_name = create_handler_fn_name(origin_name);
    let method_name = Ident::new(method, Span::call_site());
    quote! {
        pub fn #handler_modifier_fn_name(h:&mut http_server::handler::HandlerTire) {

            #handler_fn

            h.#method_name(#route, #handler_fn_name);
        }
    }
}

fn modify_fn_name(f: &mut ItemFn, name: &str) {
    let name = format!("{}_origin", name);
    f.sig.ident = Ident::new(&name, Span::call_site());
}

pub fn expand_macro(mut f: ItemFn, route: LitStr, method: &str) -> TokenStream {
    let name = f.sig.ident.to_string();
    add_return_type(&mut f);
    modify_fn_name(&mut f, name.as_str());

    let handler_fn = handler_fn(&mut f, name.as_str());
    let handler_modifier_fn = handler_modifier_fn(handler_fn, route, method, name.as_str());
    println!("{}",handler_modifier_fn);
    quote! {
        #handler_modifier_fn
    }
}
