//! Procedural macro implementations used by the `tribles` crate.
//!
//! The macros here mirror the declarative macros defined in
//! [`src/namespace.rs`](https://github.com/tribles/tribles-rust/blob/main/src/namespace.rs)
//! but are implemented with `proc_macro` to allow more complex analysis and
//! additional features in the future.  The crate currently exposes two macros:
//! [`pattern!`], which expands namespace patterns into an
//! [`IntersectionConstraint`] of query constraints, and [`entity!`], which
//! constructs [`TribleSet`]s from namespace field assignments or inserts
//! triples into an existing set.
//!
//! ```ignore
//! ::tribles_macros::pattern!(::tribles, my_ns, &set, [ { field: (42) } ]);
//! ::tribles_macros::entity!(::tribles, my_ns, { field: 42 });
//! ::tribles_macros::entity!(::tribles, my_ns, &mut set, id, { field: 42 });
//! ```
//!
//! The `pattern` macro expects the crate path, a namespace module, a dataset
//! expression implementing [`TriblePattern`], and a bracketed list of entity
//! patterns. Each entity pattern may specify an identifier using `ident @` or
//! `(expr) @` notation and contains `field: value` pairs. Values can either
//! reference an existing query variable or be written as `(expr)` to match a
//! literal.
//!
//! The `entity` macro similarly starts with the crate and namespace paths and
//! optionally an explicit entity ID expression before the field list.
//!
//! These macros are internal implementation details and should not be used
//! directly outside of the `tribles` codebase.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{braced, bracketed, parenthesized, Token};
use syn::{Expr, Ident, Path};

/// Parsed input for the [`pattern`] macro.
///
/// The invocation has the form `crate_path, namespace_path, dataset, [ .. ]`.
/// Each item in the bracketed list is parsed into an [`Entity`].
struct MacroInput {
    crate_path: Path,
    ns: Path,
    set: Expr,
    pattern: Vec<Entity>,
}

/// Description of a single entity pattern.
///
/// `id` stores the optional identifier on the left-hand side of the `@` sign.
/// `fields` holds all `name: value` constraints within the braces.
struct Entity {
    id: Option<EntityId>,
    fields: Vec<Field>,
}

/// Identifier for an [`Entity`].
enum EntityId {
    /// Use an existing variable.
    Var(Ident),
    /// Use a literal expression, e.g. `(id)`.
    Lit(Expr),
}

/// One `name: value` pair.
struct Field {
    name: Ident,
    value: FieldValue,
}

/// Value of a field pattern.
enum FieldValue {
    /// Bind the field to an existing variable.
    Var(Expr),
    /// Match the field against a literal expression.
    Lit(Expr),
}

impl Parse for MacroInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let crate_path: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let ns: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let set: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        bracketed!(content in input);
        let mut pattern = Vec::new();
        while !content.is_empty() {
            pattern.push(content.parse()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(MacroInput {
            crate_path,
            ns,
            set,
            pattern,
        })
    }
}

impl Parse for Entity {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        braced!(content in input);
        let ahead = content.fork();
        let id = if ahead.peek(Ident) && ahead.peek2(Token![@]) {
            let ident: Ident = content.parse()?;
            content.parse::<Token![@]>()?;
            Some(EntityId::Var(ident))
        } else if ahead.peek(syn::token::Paren) && ahead.peek2(Token![@]) {
            let expr_content;
            parenthesized!(expr_content in content);
            let expr: Expr = expr_content.parse()?;
            content.parse::<Token![@]>()?;
            Some(EntityId::Lit(expr))
        } else {
            None
        };
        let mut fields = Vec::new();
        while !content.is_empty() {
            fields.push(content.parse()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(Entity { id, fields })
    }
}

impl Parse for Field {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = if input.peek(syn::token::Paren) {
            let inner;
            parenthesized!(inner in input);
            FieldValue::Lit(inner.parse()?)
        } else {
            FieldValue::Var(input.parse()?)
        };
        Ok(Field { name, value })
    }
}

/// Procedural implementation of the `pattern!` macro.
///
/// This expands the namespace pattern syntax into a series of
/// [`Constraint`](::tribles::query::Constraint) objects that are joined via an
/// [`IntersectionConstraint`].
#[proc_macro]
pub fn pattern(input: TokenStream) -> TokenStream {
    match pattern_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

/// Parse the input token stream and build the constraint generating code.
fn pattern_impl(input: TokenStream) -> syn::Result<TokenStream> {
    // Parse the outer macro invocation into the typed `MacroInput` structure.
    let MacroInput {
        crate_path,
        ns,
        set,
        pattern,
    } = syn::parse(input)?;

    // Names for the generated context and dataset variables.
    let ctx_ident = format_ident!("__ctx", span = Span::call_site());
    let set_ident = format_ident!("__set", span = Span::call_site());

    // Accumulate the token stream for each entity pattern.
    let mut entity_tokens = TokenStream2::new();
    // Token stream that initializes attribute variables once.
    let mut attr_tokens = TokenStream2::new();
    // Bring the namespace into scope for attribute initialization.
    attr_tokens.extend(quote! { use #ns as ns; });
    // Counter to create unique identifiers for entity variables.
    let mut entity_idx = 0usize;
    // Counter and map for unique attribute variables.
    let mut attr_idx = 0usize;
    use std::collections::HashMap;
    let mut attr_map: HashMap<String, Ident> = HashMap::new();

    // Expand one block per entity described in the pattern.
    for entity in pattern {
        // Variable name representing the entity id.
        let e_ident = format_ident!("__e{}", entity_idx, span = Span::call_site());
        entity_idx += 1;
        // Initialization depends on whether an id was supplied.
        let init = match entity.id {
            // Existing identifier variable: reuse it directly.
            Some(EntityId::Var(id)) => quote! { let #e_ident = #id; },
            // Literal expression: create a new variable bound to the value.
            Some(EntityId::Lit(expr)) => quote! {
                let #e_ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                constraints.push({ let e: #crate_path::id::Id = #expr; Box::new(#e_ident.is(#crate_path::value::ToValue::to_value(e)))});
            },
            // No id specified: create a fresh variable for the entity.
            None => quote! {
                let #e_ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
            },
        };
        entity_tokens.extend(init);
        // Emit triple constraints for each field within the entity.
        for Field { name, value } in entity.fields {
            let field_ident = name;

            // Reuse the same attribute variable for each unique field name.
            let a_var_ident = attr_map
                .entry(field_ident.to_string())
                .or_insert_with(|| {
                    let ident = format_ident!("__a{}", attr_idx, span = Span::call_site());
                    attr_idx += 1;
                    attr_tokens.extend(quote! {
                        let #ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                        constraints.push(Box::new(#ident.is(#crate_path::value::ToValue::to_value(ns::ids::#field_ident))));
                    });
                    ident
                })
                .clone();

            let triple_tokens = match value {
                // Literal value: create a variable bound to the literal and match it.
                FieldValue::Lit(expr) => {
                    quote! {
                        {
                            use #crate_path::query::TriblePattern;
                            use #ns as ns;
                            let v_var: #crate_path::query::Variable<ns::schemas::#field_ident> = #ctx_ident.next_variable();
                            // literal value converted to a `Value`
                            let v: #crate_path::value::Value<ns::schemas::#field_ident> = #crate_path::value::ToValue::to_value(#expr);
                            // ensure the literal matches the variable
                            constraints.push(Box::new(v_var.is(v)));
                            // match the triple from the dataset
                            constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                        }
                    }
                }
                // Variable value: only emit pattern matching code.
                FieldValue::Var(expr) => {
                    quote! {
                        {
                            use #crate_path::query::TriblePattern;
                            use #ns as ns;
                            let v_var: #crate_path::query::Variable<ns::schemas::#field_ident> = #expr;
                            constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                        }
                    }
                }
            };
            entity_tokens.extend(triple_tokens);
        }
    }

    // Wrap all collected constraints in an intersection constraint
    let output = quote! {
        {
            let mut constraints: Vec<Box<dyn #crate_path::query::Constraint>> = vec![];
            let #ctx_ident = __local_find_context!();
            let #set_ident = #set;
            #attr_tokens
            #entity_tokens
            #crate_path::query::intersectionconstraint::IntersectionConstraint::new(constraints)
        }
    };

    Ok(output.into())
}

/// Parsed input for the [`entity`] macro.
///
/// Invocation forms:
/// `crate_path, namespace_path, { field: value, ... }`
/// `crate_path, namespace_path, id_expr, { field: value, ... }`
/// `crate_path, namespace_path, set_expr, id_expr, { field: value, ... }`
struct EntityInput {
    crate_path: Path,
    ns: Path,
    set: Option<Expr>,
    id: Option<Expr>,
    fields: Vec<(Ident, Expr)>,
}

impl Parse for EntityInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let crate_path: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let ns: Path = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut set = None;
        let mut id = None;

        if input.peek(syn::token::Brace) {
            // no id, no set
        } else {
            let expr1: Expr = input.parse()?;
            input.parse::<Token![,]>()?;
            if input.peek(syn::token::Brace) {
                id = Some(expr1);
            } else {
                set = Some(expr1);
                let id_expr: Expr = input.parse()?;
                input.parse::<Token![,]>()?;
                id = Some(id_expr);
            }
        }

        let content;
        braced!(content in input);
        let mut fields = Vec::new();
        while !content.is_empty() {
            let name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let value: Expr = content.parse()?;
            fields.push((name, value));
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(EntityInput {
            crate_path,
            ns,
            set,
            id,
            fields,
        })
    }
}

/// Procedural implementation of the `entity!` macro.
#[proc_macro]
pub fn entity(input: TokenStream) -> TokenStream {
    match entity_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

fn entity_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let EntityInput {
        crate_path,
        ns,
        set,
        id,
        fields,
    } = syn::parse(input)?;

    let (set_init, set_expr) = if let Some(s) = &set {
        (TokenStream2::new(), quote! { #s })
    } else {
        (
            quote! { let mut set = #crate_path::trible::TribleSet::new(); },
            quote! { set },
        )
    };

    let (id_init, id_expr) = if let Some(expr) = id {
        (
            quote! { let id_ref: &#crate_path::id::ExclusiveId = #expr; },
            quote! { id_ref },
        )
    } else {
        (
            quote! {
                let id_tmp: #crate_path::id::ExclusiveId = #crate_path::id::rngid();
                let id_ref: &#crate_path::id::ExclusiveId = &id_tmp;
            },
            quote! { id_ref },
        )
    };

    let mut insert_tokens = TokenStream2::new();
    for (field, value) in fields {
        let stmt = quote! {
            {
                use #ns as ns;
                let v: #crate_path::value::Value<ns::schemas::#field> =
                    #crate_path::value::ToValue::to_value(#value);
                #set_expr.insert(&#crate_path::trible::Trible::new(#id_expr, &ns::ids::#field, &v));
            }
        };
        insert_tokens.extend(stmt);
    }

    let output = if set.is_some() {
        quote! {{
            #id_init
            #insert_tokens
        }}
    } else {
        quote! {{
            #set_init
            #id_init
            #insert_tokens
            set
        }}
    };

    Ok(output.into())
}
