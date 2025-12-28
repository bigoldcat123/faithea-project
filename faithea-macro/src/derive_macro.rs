use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Type};

fn is_option(ty: &Type) -> bool {
    if let Type::Path(p) = ty
        && let Some(seg) = p.path.segments.last()
    {
        return seg.ident == "Option";
    }
    false
}

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
        let field_name = field_ident.to_string();
        let ty = &f.ty;

        if is_option(ty) {
            quote! {
                #field_ident: match data.remove(#field_name) {
                    Some(s) => match s.try_convert_into() {
                        Ok(s) => Some(s),
                        Err(_) => None,
                    },
                    None => None,
                }
            }
        } else {
            quote! {
                #field_ident: data
                    .remove(#field_name)
                    .ok_or_else(|| Box::new(format!("missing field `{}`", #field_name)) as faithea::handler::HttpHandlerError)?
                    .try_convert_into()?
            }
        }
    });

    Ok(quote! {
        impl faithea::data::inbound::multipart::TryFromMultipartDataMap for #struct_name {
            fn try_from_multipart_data_map(
                data: &mut std::collections::HashMap<
                    String,
                    Vec<faithea::data::inbound::multipart::Part>,
                >,
            ) -> Result<Self, faithea::handler::HttpHandlerError> {
                use faithea::TryConvertInto;
                Ok(Self {
                    #(#assigns,)*
                })
            }
        }
    }
    .into())
}
