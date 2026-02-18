use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Extract, attributes(extract))]
pub fn derive_extract(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    name,
                    "Extract only works on structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Extract only works on structs")
                .to_compile_error()
                .into();
        }
    };

    let field_extractions: Vec<_> = match fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();

            let extract_attr = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("extract"))
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        field,
                        format!("Field `{}` missing #[extract] attribute", field_name),
                    )
                })?;

            let (selector, attr_name) = parse_extract_attr(extract_attr)?;

            Ok(if attr_name == "text" {
                quote! {
                    #field_name: el.select_one(#selector)?.text()
                }
            } else {
                quote! {
                    #field_name: el.select_one(#selector)?.attr(#attr_name)?.to_string()
                }
            })
        })
        .collect::<Result<Vec<_>, syn::Error>>()
    {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let expanded = quote! {
        impl ripr::pipelines::scrape::Extract for #name {
            fn extract(el: ripr::Element) -> Option<Self> {
                Some(Self {
                    #(#field_extractions,)*
                })
            }
        }
    };

    TokenStream::from(expanded)
}

fn parse_extract_attr(attr: &Attribute) -> Result<(String, String), syn::Error> {
    let mut selector = None;
    let mut attr_name = None;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("selector") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            selector = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("attr") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            attr_name = Some(s.value());
            Ok(())
        } else {
            Err(meta.error("expected `selector` or `attr`"))
        }
    })?;

    let selector = selector
        .ok_or_else(|| syn::Error::new_spanned(attr, "Missing `selector` in #[extract]"))?;
    let attr_name =
        attr_name.ok_or_else(|| syn::Error::new_spanned(attr, "Missing `attr` in #[extract]"))?;

    Ok((selector, attr_name))
}
