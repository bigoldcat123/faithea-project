use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    FnArg, Ident, ItemFn, LitStr, Pat, PatIdent, Type, TypePath, parse_quote,
    punctuated::Punctuated, token::Comma,
};

pub fn add_return_type(f: &mut ItemFn) {
    f.sig.output = syn::parse_quote! {
        -> impl http_server::response::HttpResponseModifier
    }
}
pub enum FromHttpRequest {
    PathParam(LitStr),
    _SearchParam(LitStr),
    _Shared(LitStr),
    Body,
}
impl FromHttpRequest {
    pub fn into_token_stream(self) -> TokenStream {
        use FromHttpRequest::*;
        match self {
            PathParam(name) => {
                quote! {
                    _req.get_pathparam(#name).ok_or(format!("no such pathParam named <{}>",#name))?.convert()?,
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
            _SearchParam(name) => {
                quote! {
                    _req.get_search_param(#name).ok_or(format!("no such searchParam named <{}>",#name))?.convert()?,
                }
            }
        }
    }
}

pub fn generate_from_httprequest_list(args: &mut Punctuated<FnArg, Comma>) -> Vec<FromHttpRequest> {
    args.iter_mut().filter_map(|arg| parse_arg(arg)).collect()
}

fn parse_arg(arg: &mut FnArg) -> Option<FromHttpRequest> {
    if let Some((name, ty,is_search_param)) = extract_ident_and_type(arg) {
        // println!("{} {} {}",quote! {#name}.to_string(),quote!{#ty}.to_string(),is_search_param);
        let outer = outer_type_name(ty);
        Some(
            match outer.as_deref() {
                Some("Json") => FromHttpRequest::Body,
                Some("Shared") => FromHttpRequest::_Shared(name),
                _ => {
                    if is_search_param {
                        FromHttpRequest::_SearchParam(name)
                    }else {
                        FromHttpRequest::PathParam(name)
                    }
                },
            }
        )
    }else {
        None
    }
}

/// 提取参数名 (LitStr) 和 类型 (Type)
fn extract_ident_and_type(arg: &mut FnArg) -> Option<(LitStr, &Type,bool)> {
    if let FnArg::Typed(t) = arg {
        let mut is_search_param = false ;
        for i in 0..t.attrs.len() {
            let x = &t.attrs[i];
            let a = &x.meta;
            let name = quote! {#a}.to_string();
            if name == "search_param" {
                is_search_param = true;
                t.attrs.remove(0);
                break;
            }
        }

        if let Pat::Ident(PatIdent { ident, .. }) = t.pat.as_ref() {

            let name = LitStr::new(&ident.to_string(), ident.span());
            return Some((name, t.ty.as_ref(),is_search_param));
        }
    }
    None
}

/// 获取类型外层名字：Json<T> → Some("Json")
fn outer_type_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(TypePath { path, .. }) => path.segments.last().map(|seg| seg.ident.to_string()),
        _ => None,
    }
}


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

            res_modifier.modify(&mut res).await?;
            Ok(res)
        }
    }
}
fn add_req_param(f: &mut ItemFn) {
    f.sig
        .inputs
        .push(parse_quote!(_req:http_server::request::HttpRequest));
}
pub fn handler_fn(f: &mut ItemFn, name: &str,ipt_args:Vec<FromHttpRequest>) -> TokenStream {

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
        pub fn #handler_modifier_fn_name(h:&mut http_server::handler::HandlerTire,pre_fix:&str) {

            #handler_fn
            let mut r = format!("{}{}",pre_fix,#route);
            if r.ends_with("/") {
                r.pop();
            }
            println!("-> {:?}",r);
            h.#method_name(r, #handler_fn_name);
        }
    }
}

fn modify_fn_name(f: &mut ItemFn, name: &str) {
    let name = format!("{}_origin", name);
    f.sig.ident = Ident::new(&name, Span::call_site());
}
fn check(route: &LitStr,args:&Vec<FromHttpRequest>) -> Option<TokenStream> {
    let mut args: Vec<String> = args
        .into_iter()
        .filter_map(|x| {
            if let FromHttpRequest::PathParam(name) = x {
                Some(name.value())
            } else {
                None
            }
        })
        .collect();
    let mut path_component = route
        .value()
        .split("/")
        .filter_map(|x| {
            if x.starts_with("{") && x.ends_with("}") {
                Some(x[1..x.len() - 1].to_string())
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    args.sort();
    path_component.sort();
    if path_component.len() >= args.len() {
        for i in 0..path_component.len() {
            if i >= args.len() || path_component[i] != args[i] {
                return Some(
                    syn::Error::new(
                        route.span().clone(),
                        format!(" missing <{}> in your args", path_component[i]),
                    )
                    .to_compile_error(),
                );
            }
        }
    } else {
        for i in 0..args.len() {
            if i >= path_component.len() || path_component[i] != args[i] {
                return Some(
                    syn::Error::new(
                        route.span().clone(),
                        format!(" missing <{}> in your route", args[i]),
                    )
                    .to_compile_error(),
                );
            }
        }
    }

    None
}
pub fn expand_macro(mut f: ItemFn, route: LitStr, method: &str) -> TokenStream {
    let args = generate_from_httprequest_list(&mut f.sig.inputs);
    if let Some(err) = check(&route,&args) {
        return err.into();
    }

    let name = f.sig.ident.to_string();
    add_return_type(&mut f);
    modify_fn_name(&mut f, name.as_str());

    let handler_fn = handler_fn(&mut f, name.as_str(),args);
    let handler_modifier_fn = handler_modifier_fn(handler_fn, route, method, name.as_str());
    // println!("{}",handler_modifier_fn);
    quote! {
        #handler_modifier_fn
    }
}
