use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::Attribute;
use syn::Ident;
use syn::LitStr;
use syn::Token;
use syn::Type;
use syn::Visibility;

struct FieldDef {
    attrs: Vec<Attribute>,
    vis: Option<Visibility>,
    id: LitStr,
    name: Ident,
    ty: Type,
}

struct FieldsInput {
    fields: Vec<FieldDef>,
}

impl Parse for FieldsInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // Accept the flattened token stream form: parse the tokens directly
        // from the provided ParseStream. This matches the token shape the
        // compiler typically hands us in practice.
        let content = input;
        let mut fields = Vec::new();
        while !content.is_empty() {
            let attrs = content.call(Attribute::parse_outer)?;
            // LEGACY PLACEMENT REMOVED: visibility must appear after the
            // `as` token and before the field name so the fixed-width hex ids
            // remain visually aligned. If a caller still passes a `pub` token
            // here we emit a helpful error directing them to the new form.
            if content.peek(Token![pub]) {
                // Consume the token into a Visibility so we can span the error
                // precisely.
                let v: Visibility = content.parse()?;
                return Err(syn::Error::new_spanned(
                    v,
                    "visibility must appear after `as` and before the field name (e.g. `\"...\" as pub name: Type;`)",
                ));
            }

            // We no longer support the `doc = "..."` shorthand; prefer
            // idiomatic `///` doc comments which are parsed as outer
            // attributes by `Attribute::parse_outer` above. Keeping the input
            // syntax simple reduces complexity.

            let id: LitStr = content.parse()?;
            content.parse::<Token![as]>()?;
            // Optional visibility only in the post-`as` position.
            let vis: Option<Visibility> = if content.peek(Token![pub]) {
                Some(content.parse()?)
            } else {
                None
            };
            let name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let ty: Type = content.parse()?;
            content.parse::<Token![;]>()?;
            fields.push(FieldDef {
                attrs,
                vis,
                id,
                name,
                ty,
            });
        }
        Ok(FieldsInput { fields })
    }
}

pub(crate) fn fields_impl(input: TokenStream) -> syn::Result<TokenStream> {
    // Parse the flattened token stream; this is the form we saw in practice
    // and keeps the macro strict (no extra normalization branch).
    let ts2: TokenStream2 = input.into();
    let FieldsInput { fields } = syn::parse2(ts2)?;

    let mut out: TokenStream2 = TokenStream2::new();
    for FieldDef {
        attrs,
        vis,
        id,
        name,
        ty,
    } in fields
    {
        let vis_ts = match vis {
            Some(v) => quote! { #v },
            None => quote! { pub },
        };
        out.extend(quote! {
            #(#attrs)*
            #[allow(non_upper_case_globals)]
            #vis_ts const #name: ::tribles::field::Field<#ty> = ::tribles::field::Field::from(::tribles::id::_hex_literal_hex!(#id));
        });
    }

    Ok(out.into())
}
