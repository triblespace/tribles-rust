use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, Attribute, Ident, LitStr, Path, Token, Type, Visibility};

mod kw {
    syn::custom_keyword!(namespace);
}

struct Field {
    attrs: Vec<Attribute>,
    id: LitStr,
    name: Ident,
    ty: Type,
}

struct NamespaceInput {
    crate_path: Path,
    attrs: Vec<Attribute>,
    vis: Visibility,
    mod_name: Ident,
    fields: Vec<Field>,
}

impl Parse for NamespaceInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let crate_path: Path = input.parse()?;
        input.parse::<Token![,]>()?;

        let attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        input.parse::<kw::namespace>()?;
        let mod_name: Ident = input.parse()?;

        let content;
        braced!(content in input);
        let mut fields = Vec::new();
        while !content.is_empty() {
            let f_attrs = content.call(Attribute::parse_outer)?;
            let id: LitStr = content.parse()?;
            content.parse::<Token![as]>()?;
            let name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let ty: Type = content.parse()?;
            content.parse::<Token![;]>()?;
            fields.push(Field {
                attrs: f_attrs,
                id,
                name,
                ty,
            });
        }

        Ok(NamespaceInput {
            crate_path,
            attrs,
            vis,
            mod_name,
            fields,
        })
    }
}

pub(crate) fn namespace_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let NamespaceInput {
        crate_path,
        attrs,
        vis,
        mod_name,
        fields,
    } = syn::parse(input)?;

    let desc_fields = fields.iter().map(|Field { id, name, ty, .. }| {
        quote! {
            {
                let e = #crate_path::id::Id::new(#crate_path::namespace::hex_literal::hex!(#id)).unwrap();
                let value_schema_id = #crate_path::value::schemas::genid::GenId::value_from(<#ty as #crate_path::value::ValueSchema>::VALUE_SCHEMA_ID);
                set.insert(&#crate_path::trible::Trible::force(&e, &#crate_path::metadata::ATTR_VALUE_SCHEMA, &value_schema_id));
                if let Some(blob_schema_id) = <#ty as #crate_path::value::ValueSchema>::BLOB_SCHEMA_ID {
                    let blob_schema_id = #crate_path::value::schemas::genid::GenId::value_from(blob_schema_id);
                    set.insert(&#crate_path::trible::Trible::force(&e, &#crate_path::metadata::ATTR_BLOB_SCHEMA, &blob_schema_id));
                }
                let attr_name = #crate_path::value::schemas::shortstring::ShortString::value_from(stringify!(#name));
                set.insert(&#crate_path::trible::Trible::force(&e, &#crate_path::metadata::ATTR_NAME, &attr_name));
            }
        }
    });

    let ids_consts = fields.iter().map(|Field { attrs, id, name, .. }| {
        quote! { #(#attrs)* pub const #name: #crate_path::id::Id = #crate_path::id::Id::new(#crate_path::namespace::hex_literal::hex!(#id)).unwrap(); }
    });

    let schema_types = fields.iter().map(
        |Field {
             attrs, name, ty, ..
         }| {
            quote! { #(#attrs)* pub type #name = #ty; }
        },
    );

    let entity_macro = quote! {
        #[macro_pub::macro_pub]
        macro_rules! entity {
            ($entity:tt) => {{
                ::tribles_macros::entity!(::tribles, #mod_name, $entity)
            }};
            ($entity_id:expr, $entity:tt) => {{
                ::tribles_macros::entity!(::tribles, #mod_name, $entity_id, $entity)
            }};
        }
    };

    let pattern_macro = quote! {
        #[macro_pub::macro_pub]
        macro_rules! pattern {
            ($set:expr, $pattern: tt) => {{
                ::tribles_macros::pattern!(::tribles, #mod_name, $set, $pattern)
            }};
        }
    };

    let pattern_changes_macro = quote! {
        #[macro_pub::macro_pub]
        macro_rules! pattern_changes {
            ($curr:expr, $changes:expr, $pattern: tt) => {{
                ::tribles_macros::pattern_changes!(::tribles, #mod_name, $curr, $changes, $pattern)
            }};
        }
    };

    let output = quote! {
        #(#attrs)*
        #vis mod #mod_name {
            #![allow(unused)]
            use super::*;

            pub fn description() -> #crate_path::trible::TribleSet {
                use #crate_path::value::ValueSchema;

                let mut set = #crate_path::trible::TribleSet::new();
                #(#desc_fields)*
                set
            }
            pub mod ids {
                #![allow(non_upper_case_globals, unused)]
                use super::*;
                #(#ids_consts)*
            }
            pub mod schemas {
                #![allow(non_camel_case_types, unused)]
                use super::*;
                #(#schema_types)*
            }
            #entity_macro
            #pattern_macro
            #pattern_changes_macro
        }
    };

    Ok(output.into())
}
