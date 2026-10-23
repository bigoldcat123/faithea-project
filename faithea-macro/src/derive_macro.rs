use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Error, Expr, Fields, Lit};

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
        let field_name = extract_rename(&f.attrs).unwrap_or(field_ident.to_string());
        quote! {
            #field_ident:
            TryFromParts::try_from_parts(data
                .remove(#field_name)
                )?

        }
    });

    Ok(quote! {
        impl faithea::data::inbound::multipart::TryFromMultipartDataMap for #struct_name {
            fn try_from_multipart_data_map(
                data: &mut std::collections::HashMap<
                    String,
                    Vec<faithea::data::inbound::multipart::Part>,
                >,
            ) -> Result<Self, faithea::data::inbound::multipart::MultipartError> {
                use faithea::TryConvertInto;
                use faithea::data::inbound::multipart::TryFromParts;
                Ok(Self {
                    #(#assigns,)*
                })
            }
        }
    }
    .into())
}

// #[faithea(rename="newName")]
fn extract_rename(attr: &Vec<Attribute>) -> Option<String> {
    for a in attr {
        let m = &a.meta;
        let m_name = quote! {#m}.to_string();
        if m_name.starts_with("faithea") {
            let a = a.parse_args::<Expr>().unwrap();
            if let Expr::Assign(assignment) = a
                && let Expr::Path(left) = assignment.left.as_ref()
                && let Expr::Lit(right) = assignment.right.as_ref()
                && let Some(rename) = left.path.get_ident()
                && rename == "rename"
                && let Lit::Str(l) = &right.lit
            {
                return Some(l.value().clone());
            }
            break;
        }
    }
    None
}
