
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    FnArg, Ident, ItemFn, LitStr, Pat, Type, punctuated::Punctuated, token::Comma
};

pub fn add_return_type(f: &mut ItemFn) {
    f.sig.output = syn::parse_quote! {
        -> impl http_server::response::HttpResponseModifier
    }
}
pub enum FromHttpRequest {
    PathParam(LitStr),
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
        }
    }
}

pub fn generate_from_httprequest_list(ipt: &Punctuated<FnArg, Comma>) -> Vec<FromHttpRequest> {
    let a = ipt
        .iter()
        .map(|i| {
            if let FnArg::Typed(t) = i {
                let n = &t.pat;

                // println!("{} -> ",quote! {#n});
                let ty = &t.ty;
                let a = ty.as_ref();
                if let Type::Path(p) = a {
                    let p = &p.path;
                    let mut seg = vec![];
                    for p in p.segments.iter() {
                        let p = &p.ident;
                        seg.push(p);
                        // println!("{}",quote! {#p});
                    }
                    if let Some(s) = seg.last() {
                        if **s == Ident::new("Json", s.span()) {
                            return FromHttpRequest::Body;
                        }
                    }
                    return FromHttpRequest::PathParam(match n.as_ref() {
                        Pat::Ident(i) => {
                            let name = quote! {#i}.to_string();
                            let a = LitStr::new(name.as_str(), Span::call_site());
                            a
                        }
                        _ => {
                            unreachable!()
                        }
                    });
                }
            }
            unreachable!()
        })
        .collect::<Vec<FromHttpRequest>>();
    a
}

fn create_handler_fn_name(sig:&str) -> Ident{
    let new_fn_name = format!("{}_handler", sig);
    Ident::new(&new_fn_name, Span::call_site())
}
fn conbine_outter_fn(f:&ItemFn,args: Vec<FromHttpRequest>, orign_name: &str) -> TokenStream {
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
            )
            .await;

            res_modifier.modify(&mut res)?;
            Ok(res)
        }
    }
}

pub fn handler_fn(f:&ItemFn,name:&str) -> TokenStream {
    let ipt_args = generate_from_httprequest_list(&f.sig.inputs);
    conbine_outter_fn(f,ipt_args, name)
}

pub fn handler_modifier_fn(handler_fn:TokenStream,route:LitStr,method:&str,origin_name:&str) -> TokenStream {
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

fn modify_fn_name(f:&mut ItemFn,name:&str) {
    let name = format!("{}_origin",name);
    f.sig.ident = Ident::new(&name, Span::call_site());
}

pub fn expand_macro(mut f:ItemFn,route:LitStr,method:&str) -> TokenStream {
    let name = f.sig.ident.to_string();
    add_return_type(&mut f);
    modify_fn_name(&mut f ,name.as_str());
    let handler_fn = handler_fn(&f,name.as_str());
    let handler_modifier_fn = handler_modifier_fn(handler_fn,route,method,name.as_str());
    // println!("{}",handler_modifier_fn);
    quote! {
        #handler_modifier_fn
    }
}
