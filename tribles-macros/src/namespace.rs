use proc_macro::TokenStream;
use quote::quote;
use syn::braced;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::Attribute;
use syn::Ident;
use syn::LitStr;
use syn::Path;
use syn::Token;
use syn::Type;
use syn::Visibility;

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
                let e = #crate_path::id::Id::new(#crate_path::id::_hex_literal_hex!(#id)).unwrap();
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

    let field_consts = fields.iter().map(|Field { attrs, id, name, ty, .. }| {
        quote! { #(#attrs)* pub const #name: #crate_path::field::Field<#ty> = #crate_path::field::Field::from(#crate_path::id::_hex_literal_hex!(#id)); }
    });

    // We no longer emit per-namespace macro_rules! wrappers here. Call sites
    // should use the global `pattern!` and `entity!` proc-macros instead. The
    // per-field convenience modules (ns::field) are still generated so the
    // global macros can reference them by path.

    let output = quote! {
        #(#attrs)*
        #vis mod #mod_name {
            #![allow(unused, non_upper_case_globals)]
            use super::*;

            pub fn description() -> #crate_path::trible::TribleSet {
                use #crate_path::value::ValueSchema;

                let mut set = #crate_path::trible::TribleSet::new();
                #(#desc_fields)*
                set
            }
            // Per-field constants
            #(#field_consts)*
        }
    };

    Ok(output.into())
}
