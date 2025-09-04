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
//! ::tribles_macros::crate::pattern!(&set, [ { my_ns::field: (42) } ]);
//! ::tribles_macros::crate::entity!({ my_ns::field: 42 });
//! ::tribles_macros::crate::entity!(id, { my_ns::field: 42 });
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
use proc_macro2::Delimiter;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::TokenTree;
use quote::format_ident;
use quote::quote;
use quote::ToTokens;
use syn::braced;
use syn::bracketed;
use syn::parenthesized;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::Expr;
use syn::Ident;
use syn::Path;
use syn::Token;

mod namespace;

#[proc_macro]
pub fn namespace(input: TokenStream) -> TokenStream {
    match namespace::namespace_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn path(input: TokenStream) -> TokenStream {
    match path_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

struct PathInput {
    crate_path: Path,
    ns: Path,
    set: Expr,
    rest: TokenStream2,
}

impl Parse for PathInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let crate_path: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let ns: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let set: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let rest: TokenStream2 = input.parse()?;
        Ok(PathInput {
            crate_path,
            ns,
            set,
            rest,
        })
    }
}

fn path_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let PathInput {
        crate_path,
        ns,
        set,
        rest,
    } = syn::parse(input)?;
    let tokens: Vec<TokenTree> = rest.into_iter().collect();
    if tokens.len() < 2 {
        return Err(syn::Error::new(
            Span::call_site(),
            "expected start, regex, end",
        ));
    }
    let start = match &tokens[0] {
        TokenTree::Ident(id) => id.clone(),
        _ => {
            return Err(syn::Error::new(
                tokens[0].span(),
                "expected start identifier",
            ))
        }
    };
    let end = match &tokens[tokens.len() - 1] {
        TokenTree::Ident(id) => id.clone(),
        _ => {
            return Err(syn::Error::new(
                tokens[tokens.len() - 1].span(),
                "expected end identifier",
            ))
        }
    };
    let regex_tokens = &tokens[1..tokens.len() - 1];

    #[derive(Clone)]
    enum Tok {
        Sym(Ident),
        Or,
        Star,
        Plus,
        LParen,
        RParen,
    }

    fn lex(ts: &[TokenTree]) -> syn::Result<Vec<Tok>> {
        let mut out = Vec::new();
        let mut i = 0usize;
        while i < ts.len() {
            match &ts[i] {
                TokenTree::Ident(id) => {
                    out.push(Tok::Sym(id.clone()));
                    i += 1;
                }
                TokenTree::Punct(p) if p.as_char() == '|' => {
                    out.push(Tok::Or);
                    i += 1;
                }
                TokenTree::Punct(p) if p.as_char() == '*' => {
                    out.push(Tok::Star);
                    i += 1;
                }
                TokenTree::Punct(p) if p.as_char() == '+' => {
                    out.push(Tok::Plus);
                    i += 1;
                }
                TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => {
                    i += 1;
                    out.push(Tok::LParen);
                    out.extend(lex(&g.stream().into_iter().collect::<Vec<_>>())?);
                    out.push(Tok::RParen);
                }
                t => return Err(syn::Error::new(t.span(), "unexpected token in regex")),
            }
        }
        Ok(out)
    }

    let lexed = lex(regex_tokens)?;

    fn needs_concat(a: &Tok, b: &Tok) -> bool {
        matches!(a, Tok::Sym(_) | Tok::RParen | Tok::Star | Tok::Plus)
            && matches!(b, Tok::Sym(_) | Tok::LParen)
    }

    #[derive(Clone)]
    enum OpTok {
        Sym(Ident),
        Or,
        Concat,
        Star,
        Plus,
        LParen,
        RParen,
    }

    let mut infix = Vec::new();
    for i in 0..lexed.len() {
        match &lexed[i] {
            Tok::Sym(p) => infix.push(OpTok::Sym(p.clone())),
            Tok::Or => infix.push(OpTok::Or),
            Tok::Star => infix.push(OpTok::Star),
            Tok::Plus => infix.push(OpTok::Plus),
            Tok::LParen => infix.push(OpTok::LParen),
            Tok::RParen => infix.push(OpTok::RParen),
        }
        if i + 1 < lexed.len() && needs_concat(&lexed[i], &lexed[i + 1]) {
            infix.push(OpTok::Concat);
        }
    }

    fn prec(t: &OpTok) -> u8 {
        match t {
            OpTok::Star | OpTok::Plus => 3,
            OpTok::Concat => 2,
            OpTok::Or => 1,
            _ => 0,
        }
    }
    fn right_assoc(t: &OpTok) -> bool {
        matches!(t, OpTok::Star | OpTok::Plus)
    }

    let mut output = Vec::<OpTok>::new();
    let mut stack = Vec::<OpTok>::new();
    for token in infix {
        match token {
            OpTok::Sym(_) => output.push(token),
            OpTok::LParen => stack.push(OpTok::LParen),
            OpTok::RParen => {
                while let Some(op) = stack.pop() {
                    if matches!(op, OpTok::LParen) {
                        break;
                    } else {
                        output.push(op);
                    }
                }
            }
            OpTok::Or | OpTok::Concat | OpTok::Star | OpTok::Plus => {
                while let Some(op) = stack.last() {
                    if matches!(op, OpTok::LParen) {
                        break;
                    }
                    if prec(op) > prec(&token) || (!right_assoc(&token) && prec(op) == prec(&token))
                    {
                        output.push(stack.pop().unwrap());
                    } else {
                        break;
                    }
                }
                stack.push(token);
            }
        }
    }
    while let Some(op) = stack.pop() {
        output.push(op);
    }

    let ops: Vec<TokenStream2> = output
        .into_iter()
        .map(|t| match t {
            OpTok::Sym(ident) => {
                quote! { PathOp::Attr(#crate_path::id::RawId::from(#ns::ids::#ident)) }
            }
            OpTok::Or => quote! { PathOp::Union },
            OpTok::Concat => quote! { PathOp::Concat },
            OpTok::Star => quote! { PathOp::Star },
            OpTok::Plus => quote! { PathOp::Plus },
            _ => panic!(),
        })
        .collect();

    let output = quote! {
        {
            use #crate_path::query::regularpathconstraint::{PathOp, RegularPathConstraint, ThompsonEngine};
            RegularPathConstraint::<ThompsonEngine>::new(#set.clone(), #start, #end, &[#(#ops),*])
        }
    };
    Ok(output.into())
}

/// Parsed input for the [`pattern`] macro.
///
/// The invocation has the form `crate_path, namespace_path, dataset, [ .. ]`.
/// Each item in the bracketed list is parsed into an [`Entity`].
struct MacroInput {
    // Optional crate path / namespace form (legacy). If absent the macro was
    // invoked in the simplified form `pattern!(<set>, [ ... ])` where field
    // names are full paths (or brought into scope with `use`).
    crate_path: Option<Path>,
    ns: Option<Path>,
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
    // Allow the field name to be a full path (e.g. `literature::firstname`)
    // or a single identifier (e.g. `firstname`). The macro will append
    // `::id` / `::schema` to this path when generating code, and when the
    // legacy `ns` argument is present a single-segment name will be
    // interpreted as `ns::name` for backward compatibility.
    name: Path,
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
        // Simplified form: <set>, [ ... ]
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
            crate_path: None,
            ns: None,
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
        let name: Path = input.parse()?;
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

    // Compute a crate path token stream to reference the host `tribles` crate
    // in generated code. If the caller provided an explicit crate path (the
    // legacy form) use that; otherwise default to `::tribles` for the
    // simplified invocation exported via the prelude.
    let crate_path_ts: TokenStream2 = quote! { ::tribles };

    // Shadow the parsed `crate_path` with a TokenStream2 for use inside
    // `quote!` expansions as `#crate_path`.
    let crate_path = crate_path_ts.clone();
    // Shadow the original `crate_path` (Option) with a TokenStream2 that the
    // quoting machinery can interpolate directly as `#crate_path`.
    let crate_path = crate_path_ts.clone();

    // Accumulate the token stream for each entity pattern.
    let mut entity_tokens = TokenStream2::new();
    // Token stream that initializes attribute variables once.
    let mut attr_tokens = TokenStream2::new();
    // If a namespace was provided (legacy invocation) bring it into scope
    // for the generated code. For the simplified invocation (no `ns`), we
    // expect callers to supply fully-qualified field module paths or import
    // them with `use`.
    if let Some(ns_path) = &ns {
        attr_tokens.extend(quote! { #[allow(unused_imports)] use #ns_path as ns; });
    }
    // Counter to create unique identifiers for entity variables.
    // Counter and map for unique attribute variables.
    let mut attr_idx = 0usize;
    use std::collections::HashMap;
    let mut attr_map: HashMap<String, Ident> = HashMap::new();

    // Expand one block per entity described in the pattern.
    for (entity_idx, entity) in pattern.into_iter().enumerate() {
        // Variable name representing the entity id.
        let e_ident = format_ident!("__e{}", entity_idx, span = Span::call_site());
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
            let field_path = name;

            // Decide whether a single-segment path should be interpreted as
            // relative to the provided namespace (legacy form). For the new
            // simplified invocation the `ns` will be None and the caller is
            // expected to supply a fully-qualified path or bring the module
            // into scope with `use`.
            let single_segment = field_path.leading_colon.is_none() && field_path.segments.len() == 1;

            // Build a stable string key for attribute reuse.
            let key = if single_segment {
                format!("{}::{}", ns.to_token_stream(), field_path.to_token_stream())
            } else {
                field_path.to_token_stream().to_string()
            };

            // Reuse the same attribute variable for each unique field path.
            let a_var_ident = attr_map
                .entry(key)
                .or_insert_with(|| {
                    let ident = format_ident!("__a{}", attr_idx, span = Span::call_site());
                    attr_idx += 1;
                    // Emit attribute variable initialization referencing the
                    // appropriate per-field `::id` constant.
                    if let Some(ns_path) = &ns {
                        if single_segment {
                            attr_tokens.extend(quote! {
                                let #ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                                constraints.push(Box::new(#ident.is(#crate_path::value::ToValue::to_value(#ns_path::#field_path::id))));
                            });
                        } else {
                            attr_tokens.extend(quote! {
                                let #ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                                constraints.push(Box::new(#ident.is(#crate_path::value::ToValue::to_value(#field_path::id))));
                            });
                        }
                    } else {
                        attr_tokens.extend(quote! {
                            let #ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                            constraints.push(Box::new(#ident.is(#crate_path::value::ToValue::to_value(#field_path::id))));
                        });
                    }
                    ident
                })
                .clone();

            let triple_tokens = match value {
                // Literal value: create a variable bound to the literal and match it.
                FieldValue::Lit(expr) => {
                    if let Some(ns_path) = &ns {
                        if single_segment {
                            quote! {
                                {
                                    #[allow(unused_imports)] use #crate_path::query::TriblePattern;
                                    let v_var: #crate_path::query::Variable<#ns_path::#field_path::schema> = #ctx_ident.next_variable();
                                    // literal value converted to a `Value`
                                    let v: #crate_path::value::Value<#ns_path::#field_path::schema> = #crate_path::value::ToValue::to_value(#expr);
                                    // ensure the literal matches the variable
                                    constraints.push(Box::new(v_var.is(v)));
                                    // match the triple from the dataset
                                    constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                                }
                            }
                        } else {
                            quote! {
                                {
                                    #[allow(unused_imports)] use #crate_path::query::TriblePattern;
                                    let v_var: #crate_path::query::Variable<#field_path::schema> = #ctx_ident.next_variable();
                                    let v: #crate_path::value::Value<#field_path::schema> = #crate_path::value::ToValue::to_value(#expr);
                                    constraints.push(Box::new(v_var.is(v)));
                                    constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                                }
                            }
                        }
                    } else {
                        quote! {
                            {
                                #[allow(unused_imports)] use #crate_path::query::TriblePattern;
                                let v_var: #crate_path::query::Variable<#field_path::schema> = #ctx_ident.next_variable();
                                let v: #crate_path::value::Value<#field_path::schema> = #crate_path::value::ToValue::to_value(#expr);
                                constraints.push(Box::new(v_var.is(v)));
                                constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                            }
                        }
                    }
                }
                // Variable value: only emit pattern matching code.
                FieldValue::Var(expr) => {
                    if let Some(ns_path) = &ns {
                        if single_segment {
                            quote! {
                                {
                                    #[allow(unused_imports)] use #crate_path::query::TriblePattern;
                                    let v_var: #crate_path::query::Variable<#ns_path::#field_path::schema> = #expr;
                                    constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                                }
                            }
                        } else {
                            quote! {
                                {
                                    #[allow(unused_imports)] use #crate_path::query::TriblePattern;
                                    let v_var: #crate_path::query::Variable<#field_path::schema> = #expr;
                                    constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                                }
                            }
                        }
                    } else {
                        quote! {
                            {
                                #[allow(unused_imports)] use #crate_path::query::TriblePattern;
                                let v_var: #crate_path::query::Variable<#field_path::schema> = #expr;
                                constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                            }
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
            let mut constraints: #crate_path::arrayvec::ArrayVec<Box<dyn #crate_path::query::Constraint>, 16> = #crate_path::arrayvec::ArrayVec::new();
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
    crate_path: Option<Path>,
    ns: Option<Path>,
    set: Option<Expr>,
    id: Option<Expr>,
    fields: Vec<(Path, Expr)>,
}

impl Parse for EntityInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // Try legacy form first: <crate_path>, <ns>, ...
        let fork = input.fork();
        if let Ok(cp) = fork.parse::<Path>() {
            if fork.peek(Token![,]) {
                let _ = fork.parse::<Token![,]>();
                if let Ok(ns_p) = fork.parse::<Path>() {
                    if fork.peek(Token![,]) {
                        // Legacy form consumed for real
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
                            let name: Path = content.parse()?;
                            content.parse::<Token![:]>()?;
                            let value: Expr = content.parse()?;
                            fields.push((name, value));
                            if content.peek(Token![,]) {
                                content.parse::<Token![,]>()?;
                            }
                        }

                        return Ok(EntityInput {
                            crate_path: Some(crate_path),
                            ns: Some(ns),
                            set,
                            id,
                            fields,
                        });
                    }
                }
            }
        }

        // Simplified form: [ set_expr?, id_expr? ], { fields }
        let mut set = None;
        let mut id = None;
        if input.peek(syn::token::Brace) {
            // nothing
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
            let name: Path = content.parse()?;
            content.parse::<Token![:]>()?;
            let value: Expr = content.parse()?;
            fields.push((name, value));
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(EntityInput {
            crate_path: None,
            ns: None,
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

/// Parsed input for the [`pattern_changes`] macro.
///
/// The invocation takes the form `crate_path, namespace_path, current, changes, [..]`.
/// Both dataset expressions evaluate to [`TribleSet`]s. `current` is the full
/// dataset and `changes` represents the newly inserted tribles. The pattern
/// syntax matches that of [`pattern!`].
struct PatternChangesInput {
    crate_path: Path,
    ns: Path,
    curr: Expr,
    changes: Expr,
    pattern: Vec<Entity>,
}

impl Parse for PatternChangesInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let crate_path: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let ns: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let curr: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let changes: Expr = input.parse()?;
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
        Ok(PatternChangesInput {
            crate_path,
            ns,
            curr,
            changes,
            pattern,
        })
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

    // Determine the crate path tokens to reference the host `tribles` crate.
    // If the caller provided an explicit crate path (legacy form) use that,
    // otherwise default to `::tribles` which is the common case for the
    // simplified invocation exported from the prelude.
    let crate_path_ts: TokenStream2 = quote! { ::tribles };

    let (set_init, set_expr) = if let Some(s) = &set {
        (TokenStream2::new(), quote! { #s })
    } else {
        (
            quote! { let mut set = #crate_path_ts::trible::TribleSet::new(); },
            quote! { set },
        )
    };

    let (id_init, id_expr) = if let Some(expr) = id {
        (
            quote! { let id_ref: &#crate_path_ts::id::ExclusiveId = #expr; },
            quote! { id_ref },
        )
    } else {
        (
            quote! {
                let id_tmp: #crate_path_ts::id::ExclusiveId = #crate_path_ts::id::rngid();
                let id_ref: &#crate_path_ts::id::ExclusiveId = &id_tmp;
            },
            quote! { id_ref },
        )
    };

    let mut insert_tokens = TokenStream2::new();
    for (field, value) in fields {
        let single_segment = field.leading_colon.is_none() && field.segments.len() == 1;
        let stmt = if let Some(ns_path) = &ns {
            if single_segment {
                quote! {
                    {
                        let v: #crate_path::value::Value<#ns_path::#field::schema> =
                            #crate_path::value::ToValue::to_value(#value);
                        #set_expr.insert(&#crate_path::trible::Trible::new(#id_expr, &#ns_path::#field::id, &v));
                    }
                }
            } else {
                quote! {
                    {
                        let v: #crate_path::value::Value<#field::schema> =
                            #crate_path::value::ToValue::to_value(#value);
                        #set_expr.insert(&#crate_path::trible::Trible::new(#id_expr, &#field::id, &v));
                    }
                }
            }
        } else {
            quote! {
                {
                    let v: #crate_path::value::Value<#field::schema> =
                        #crate_path::value::ToValue::to_value(#value);
                    #set_expr.insert(&#crate_path::trible::Trible::new(#id_expr, &#field::id, &v));
                }
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

/// Procedural implementation of the `pattern_changes!` macro.
#[proc_macro]
pub fn pattern_changes(input: TokenStream) -> TokenStream {
    match pattern_changes_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

fn pattern_changes_impl(input: TokenStream) -> syn::Result<TokenStream> {
    use std::collections::HashMap;

    let PatternChangesInput {
        crate_path,
        ns,
        curr,
        changes,
        pattern,
    } = syn::parse(input)?;

    // Identifiers used throughout the expansion
    let ctx_ident = format_ident!("__ctx", span = Span::call_site());
    let curr_ident = format_ident!("__curr", span = Span::call_site());
    let delta_ident = format_ident!("__delta", span = Span::call_site());

    // Prepare declarations shared by all union branches
    let mut attr_decl_tokens = TokenStream2::new();
    let mut attr_const_tokens = TokenStream2::new();

    let mut entity_decl_tokens = TokenStream2::new();
    let mut entity_const_tokens = TokenStream2::new();

    let mut value_decl_tokens = TokenStream2::new();
    let mut value_const_tokens = TokenStream2::new();

    struct TripleInfo {
        e_ident: Ident,
        a_ident: Ident,
        v_ident: Ident,
    }
    let mut triples: Vec<TripleInfo> = Vec::new();

    let mut attr_map: HashMap<String, Ident> = HashMap::new();
    let mut attr_idx = 0usize;
    let mut value_idx = 0usize;

    for (entity_idx, entity) in pattern.into_iter().enumerate() {
        let e_ident = format_ident!("__e{}", entity_idx, span = Span::call_site());
        match entity.id {
            Some(EntityId::Var(id)) => {
                entity_decl_tokens.extend(quote! { let #e_ident = #id; });
            }
            Some(EntityId::Lit(expr)) => {
                entity_decl_tokens.extend(quote! {
                    let #e_ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                });
                entity_const_tokens.extend(quote! {
                    constraints.push({ let e: #crate_path::id::Id = #expr; Box::new(#e_ident.is(#crate_path::value::ToValue::to_value(e)))});
                });
            }
            None => {
                entity_decl_tokens.extend(quote! {
                    let #e_ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                });
            }
        }

        for Field { name, value } in entity.fields {
            let field_path = name;
            let single_segment = field_path.leading_colon.is_none() && field_path.segments.len() == 1;
            let key = if single_segment {
                format!("{}::{}", ns.to_token_stream(), field_path.to_token_stream())
            } else {
                field_path.to_token_stream().to_string()
            };

            let a_ident = attr_map
                .entry(key)
                .or_insert_with(|| {
                    let ident = format_ident!("__a{}", attr_idx, span = Span::call_site());
                    attr_idx += 1;
                    attr_decl_tokens.extend(quote! {
                        let #ident: #crate_path::query::Variable<#crate_path::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                    });
                    if single_segment {
                        attr_const_tokens.extend(quote! {
                            constraints.push(Box::new(#ident.is(#crate_path::value::ToValue::to_value(#ns::#field_path::id))));
                        });
                    } else {
                        attr_const_tokens.extend(quote! {
                            constraints.push(Box::new(#ident.is(#crate_path::value::ToValue::to_value(#field_path::id))));
                        });
                    }
                    ident
                })
                .clone();

            let v_ident = format_ident!("__v{}", value_idx, span = Span::call_site());
            value_idx += 1;

            match value {
                FieldValue::Lit(expr) => {
                    let val_ident = format_ident!("__c{}", value_idx, span = Span::call_site());
                    value_idx += 1;
                    if single_segment {
                        value_decl_tokens.extend(quote! {
                            let #v_ident: #crate_path::query::Variable<#ns::#field_path::schema> = #ctx_ident.next_variable();
                            let #val_ident: #crate_path::value::Value<#ns::#field_path::schema> = #crate_path::value::ToValue::to_value(#expr);
                        });
                    } else {
                        value_decl_tokens.extend(quote! {
                            let #v_ident: #crate_path::query::Variable<#field_path::schema> = #ctx_ident.next_variable();
                            let #val_ident: #crate_path::value::Value<#field_path::schema> = #crate_path::value::ToValue::to_value(#expr);
                        });
                    }
                    value_const_tokens.extend(quote! {
                        constraints.push(Box::new(#v_ident.is(#val_ident)));
                    });
                }
                FieldValue::Var(expr) => {
                    if single_segment {
                        value_decl_tokens.extend(quote! {
                            let #v_ident: #crate_path::query::Variable<#ns::#field_path::schema> = #expr;
                        });
                    } else {
                        value_decl_tokens.extend(quote! {
                            let #v_ident: #crate_path::query::Variable<#field_path::schema> = #expr;
                        });
                    }
                }
            }

            triples.push(TripleInfo {
                e_ident: e_ident.clone(),
                a_ident: a_ident.clone(),
                v_ident,
            });
        }
    }

    let mut case_exprs: Vec<TokenStream2> = Vec::new();
    for delta_idx in 0..triples.len() {
        let mut triple_tokens = TokenStream2::new();
        for (
            idx,
            TripleInfo {
                e_ident,
                a_ident,
                v_ident,
            },
        ) in triples.iter().enumerate()
        {
            let dataset = if idx == delta_idx {
                &delta_ident
            } else {
                &curr_ident
            };
            triple_tokens.extend(quote! {
                constraints.push(Box::new(#dataset.pattern(#e_ident, #a_ident, #v_ident)));
            });
        }

        let case = quote! {
            {
                let mut constraints: #crate_path::arrayvec::ArrayVec<Box<dyn #crate_path::query::Constraint>, 16> = #crate_path::arrayvec::ArrayVec::new();
                #[allow(unused_imports)] use #crate_path::query::TriblePattern;
                #triple_tokens
                #crate_path::query::intersectionconstraint::IntersectionConstraint::new(constraints)
            }
        };
        case_exprs.push(case);
    }

    let union_expr = quote! {
        #crate_path::query::unionconstraint::UnionConstraint::new(vec![
            #(Box::new(#case_exprs) as Box<dyn #crate_path::query::Constraint>),*
        ])
    };

    let ns_use = quote! { #[allow(unused_imports)] use #ns as ns; };

    let output = quote! {
        {
            let #ctx_ident = __local_find_context!();
                        let #curr_ident = #curr;
            let #delta_ident = #changes;
            #ns_use
            #attr_decl_tokens
            #entity_decl_tokens
            #value_decl_tokens
            let mut constraints: #crate_path::arrayvec::ArrayVec<Box<dyn #crate_path::query::Constraint>, 16> = #crate_path::arrayvec::ArrayVec::new();
            #[allow(unused_imports)] use #crate_path::query::TriblePattern;
            #attr_const_tokens
            #entity_const_tokens
            #value_const_tokens
            constraints.push(Box::new(#union_expr));
            #crate_path::query::intersectionconstraint::IntersectionConstraint::new(constraints)
        }
    };

    Ok(output.into())
}
