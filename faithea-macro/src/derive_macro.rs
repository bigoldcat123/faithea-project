use proc_macro::TokenStream;
use quote::{ quote};
use syn::{Attribute, Data, DeriveInput, Error, Expr, Fields, Lit};

// fn is_option(ty: &Type) -> bool {
//     if let Type::Path(p) = ty
//         && let Some(seg) = p.path.segments.last()
//     {
//         return seg.ident == "Option";
//     }
//     false
// }

pub fn expand_multipart(input: &DeriveInput) -> Result<TokenStream, Error> {
    let struct_name = &input.ident;

    let fields = match &input.data {
        Data::Struct(s) => &s.fields,
        _ => {
            return Err(Error::new_spanned(
                input,
                "MultipartData can only be derived for structs",
            ));
        }
    };

    let named_fields = match fields {
        Fields::Named(n) => &n.named,
        _ => {
            return Err(Error::new_spanned(
                fields,
                "MultipartData only supports named fields",
            ));
        }
    };

    let assigns = named_fields.iter().map(|f| {
        let field_ident = f.ident.as_ref().unwrap();
        let field_name = extra_rename(&f.attrs).unwrap_or(field_ident.to_string());
        quote! {
            #field_ident: data
                .remove(#field_name)
                .try_convert_into()?
        }
    });

    Ok(quote! {
        impl faithea::data::inbound::multipart::TryFromMultipartDataMap for #struct_name {
            fn try_from_multipart_data_map(
                data: &mut std::collections::HashMap<
                    String,
                    Vec<faithea::data::inbound::multipart::Part>,
                >,
            ) -> Result<Self, faithea::handler::types::HttpHandlerError> {
                use faithea::TryConvertInto;
                Ok(Self {
                    #(#assigns,)*
                })
            }
        }
    }
    .into())
}

// #[faithea(rename="newName")]
fn extra_rename(attr: & Vec<Attribute>) -> Option<String> {
    for i in 0..attr.len() {
        let a = &attr[i];
        let m = &a.meta;
        let m_name = quote! {#m}.to_string();
        if m_name.starts_with("faithea") {
            let a = a.parse_args::<Expr>().unwrap();
            if let Expr::Assign(asign) = a {
                if let Expr::Path(left) = asign.left.as_ref()
                    && let Expr::Lit(right) = asign.right.as_ref()
                {
                    if let Some(rename) = left.path.get_ident() {
                        if rename == "rename" {
                            if let Lit::Str(l) = &right.lit {
                                return Some(l.value().clone())
                            }
                        }
                    }
                }
            }
            break;
        }
    }
    None
}
