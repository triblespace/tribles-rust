//! Procedural macro implementations used by the `tribles` crate.
//!
//! The macros here mirror the declarative macros defined in
//! [`src/namespace.rs`](https://github.com/tribles/tribles-rust/blob/main/src/namespace.rs)
//! but are implemented with `proc_macro` to allow more complex analysis and
//! additional features in the future.  The crate currently exposes two macros:
//! [`pattern!`], which expands namespace patterns into an
//! [`IntersectionConstraint`] of query constraints, and [`entity!`], which
//! constructs [`TribleSet`]s from namespace attribute assignments or inserts
//! triples into an existing set.
//!
//! ```ignore
//! // Match an entity whose `my_ns::attr` equals the literal 42
//! ::tribles_macros::pattern!(&set, [ { my_ns::attr: 42 } ]);
//! // Bind the value of `my_ns::attr` to the variable `v` (use `?v`)
//! ::tribles_macros::pattern!(&set, [ { my_ns::attr: ?v }]);
//! // Construct an entity set with an attribute assignment
//! ::tribles_macros::entity!{  my_ns::attr: 42  };
//! ::tribles_macros::entity!{ id @  my_ns::attr: 42  };
//! ```
//!
//! The `pattern` macro expects a dataset expression implementing
//! [`TriblePattern`] and a bracketed list of entity patterns. Each entity
//! pattern may specify an identifier using `ident @` or `(expr) @` notation and
//! contains `attribute: value` pairs. For `pattern!` a value prefixed with `?`
//! (for example `?v`) denotes a variable binding. All other values are parsed
//! as ordinary Rust expressions and treated as literal values.
//!
//! The `entity` macro similarly starts with the crate and namespace paths and
//! optionally an explicit entity ID expression before the attribute list.
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
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::Expr;
use syn::Ident;
use syn::Path;
use syn::Token;

mod attributes;

#[proc_macro]
pub fn path(input: TokenStream) -> TokenStream {
    match path_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn attributes(input: TokenStream) -> TokenStream {
    match attributes::attributes_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

struct PathInput {
    set: Expr,
    rest: TokenStream2,
}

impl Parse for PathInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // Simplified form: <set>, <start> <regex> <end>
        let set: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let rest: TokenStream2 = input.parse()?;
        Ok(PathInput { set, rest })
    }
}

fn path_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let PathInput { set, rest } = syn::parse(input)?;
    // Create a tokenized crate path for use in `quote!` macros below.
    let crate_path_ts: TokenStream2 = quote! { ::tribles };
    let _crate_path = crate_path_ts.clone();
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
        Sym(Path),
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
                // Collect one or more tokens forming a Rust path (e.g. `ns::field`).
                TokenTree::Ident(_) => {
                    // Gather consecutive Ident and ':' punctuation tokens into
                    // a single path string and parse it with `syn::Path`.
                    let mut j = i;
                    let mut pieces: Vec<String> = Vec::new();
                    while j < ts.len() {
                        match &ts[j] {
                            TokenTree::Ident(id) => {
                                pieces.push(id.to_string());
                                j += 1;
                            }
                            TokenTree::Punct(p) if p.as_char() == ':' => {
                                // include the punctuation in the textual path
                                pieces.push(p.as_char().to_string());
                                j += 1;
                            }
                            _ => break,
                        }
                    }
                    let s = pieces.join("");
                    let path: Path = syn::parse_str(&s).map_err(|e| {
                        syn::Error::new(ts[i].span(), format!("invalid path in regex: {}", e))
                    })?;
                    out.push(Tok::Sym(path));
                    i = j;
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
        Sym(Path),
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
            OpTok::Sym(path) => {
                // The Path here refers to a Field constant (e.g. `ns::attr`).
                // Call `.raw()` on the constant to obtain the underlying RawId
                // so the path matcher can operate on attribute ids directly.
                quote! { PathOp::Attr(#path.raw()) }
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
            use ::tribles::query::regularpathconstraint::{PathOp, RegularPathConstraint, ThompsonEngine};
            RegularPathConstraint::<ThompsonEngine>::new(#set.clone(), #start, #end, &[#(#ops),*])
        }
    };
    Ok(output.into())
}

/// Parsed input for the [`pattern`] macro.
///
/// The invocation has the form `crate_path, namespace_path, dataset, [ .. ]`.
/// Each item in the bracketed list is parsed into an [`Entity`].
struct PatternInput {
    set: Expr,
    pattern: Vec<Entity>,
}

/// Description of a single entity pattern.
///
/// `id` stores the optional identifier on the left-hand side of the `@` sign.
/// `attributes` holds all `name: value` constraints within the braces.
struct Entity {
    // Allow the entity id before `@` to be a variable binding `?ident`
    // or any arbitrary expression; reuse the same PatternValue/Value type
    // used for attribute values so parsing is consistent.
    id: Option<Value>,
    attributes: Vec<Attribute>,
}

/// Identifier for an [`Entity`].

/// One `name: value` pair.
/// Value used for attribute assignments in `pattern!` and related macros.
///
/// We accept an explicit `?ident` form for variable binding. Any other
/// token sequence is parsed as a normal Rust expression. This makes the
/// macro unambiguous and avoids relying on ad-hoc parenthesisation rules.
enum Value {
    /// `?ident` — bind this identifier as a query variable
    Var(Ident),
    /// Arbitrary Rust expression used as a literal value
    Expr(Expr),
}

struct Attribute {
    // The field name is an arbitrary expression that evaluates to a
    // `Field<S>` value (for some schema S). This covers local constants,
    // fully-qualified constants and inline constructors like
    // `Field::<ShortString>::from(hex!("..."))`.
    name: Expr,
    value: Value,
}

impl Parse for PatternInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // Simplified form: <set>, [ ... ]
        let set: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        bracketed!(content in input);

        let pattern = Punctuated::<_, Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect();

        Ok(PatternInput { set, pattern })
    }
}

impl Parse for Entity {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        braced!(content in input);

        // Try to parse a PatternValue (which may be `?ident` or an Expr)
        // followed by `@` as the id. Use a fork so we only consume the id
        // when `@` actually follows.
        let mut id: Option<Value> = None;
        let fork = content.fork();
        if fork.parse::<Value>().is_ok() && fork.peek(Token![@]) {
            let pv: Value = content.parse()?;
            content.parse::<Token![@]>()?;
            id = Some(pv);
        }

        let attributes = Punctuated::<_, Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect();

        Ok(Entity { id, attributes })
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Expr = input.parse()?;
        input.parse::<Token![:]>()?;
        // Peek for an explicit `?` prefix to indicate a variable binding.
        let value: Value = input.parse()?;
        Ok(Attribute { name, value })
    }
}

impl Parse for Value {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            let var_ident: Ident = input.parse()?;
            Ok(Value::Var(var_ident))
        } else {
            let expr: Expr = input.parse()?;
            Ok(Value::Expr(expr))
        }
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
    // New simplified form: <set>, [ ... ]
    let PatternInput { set, pattern } = syn::parse(input)?;

    // Names for the generated context and dataset variables.
    let ctx_ident = format_ident!("__ctx", span = Span::mixed_site());
    let set_ident = format_ident!("__set", span = Span::mixed_site());

    // Compute a crate path token stream to reference the host `tribles` crate
    // in generated code. If the caller provided an explicit crate path (the
    // legacy form) use that; otherwise default to `::tribles` for the
    // simplified invocation exported via the prelude.
    let crate_path_ts: TokenStream2 = quote! { ::tribles };

    // Shadow the parsed `crate_path` with a TokenStream2 for use inside
    // `quote!` expansions as `#crate_path`.
    let _crate_path = crate_path_ts.clone();
    // Shadow the original `crate_path` (Option) with a TokenStream2 that the
    // quoting machinery can interpolate directly as `#crate_path`.
    let _crate_path = crate_path_ts.clone();

    // Accumulate the token stream for each entity pattern.
    let mut entity_tokens = TokenStream2::new();
    // Token stream that initializes attribute variables once.
    let mut attr_tokens = TokenStream2::new();
    // For the simplified invocation callers must supply fully-qualified
    // field module paths (e.g. `literature::firstname`) or `use` them into
    // scope. We no longer accept a separate `ns` argument.
    // Counter to create unique identifiers for entity variables.
    // Counter and map for unique attribute variables. We store both the
    // attribute variable name and the evaluated field expression identifier
    // so the attribute id can be computed once per unique attribute.
    let mut attr_idx = 0usize;
    let mut val_idx = 0usize;
    use std::collections::HashMap;
    let mut attr_map: HashMap<String, (Ident, Ident)> = HashMap::new();

    // Expand one block per entity described in the pattern.
    for (entity_idx, entity) in pattern.into_iter().enumerate() {
        // Variable name representing the entity id.
        let e_ident = format_ident!("__e{}", entity_idx, span = Span::mixed_site());
        // Initialization depends on whether an id was supplied. The id may be
        // a `Value::Var(ident)` or `Value::Expr(expr)`. Treat bare variable
        // bindings similarly to single-segment path expressions for now.
        let init = if let Some(ref id_val) = entity.id {
            match id_val {
                Value::Var(ref ident) => {
                    quote! { let #e_ident = #ident; }
                }
                Value::Expr(ref id_expr) => {
                    quote! {
                        let #e_ident: ::tribles::query::Variable<::tribles::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                        constraints.push(Box::new(#e_ident.is(::tribles::value::ToValue::to_value(#id_expr))));
                    }
                }
            }
        } else {
            quote! {
                let #e_ident: ::tribles::query::Variable<::tribles::value::schemas::genid::GenId> = #ctx_ident.next_variable();
            }
        };
        entity_tokens.extend(init);
        // Emit triple constraints for each field within the entity.
        for Attribute {
            name: field_expr,
            value,
        } in entity.attributes
        {
            let key = field_expr.to_token_stream().to_string();
            let (a_var_ident, af_ident) = attr_map
                .entry(key)
                .or_insert_with(|| {
                    let a_ident = format_ident!("__a{}", attr_idx, span = Span::mixed_site());
                    let af_ident = format_ident!("__af{}", attr_idx, span = Span::mixed_site());
                    attr_idx += 1;
                    attr_tokens.extend(quote! {
                        let #af_ident = #field_expr;
                        let #a_ident: ::tribles::query::Variable<::tribles::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                        constraints.push(Box::new(#a_ident.is(::tribles::value::ToValue::to_value(#af_ident.id()))));
                    });
                    (a_ident, af_ident)
                })
                .clone();

            // Create unique identifiers for value temporaries
            let val_id = {
                let v = val_idx;
                val_idx += 1;
                v
            };
            let v_tmp_ident = format_ident!("__v{}", val_id, span = Span::mixed_site());
            let _raw_ident = format_ident!("__raw{}", val_id, span = Span::mixed_site());

            // Attribute values are represented by `PatternValue`. We accept
            // an explicit `?ident` form for variable binding; otherwise the
            // provided expression is treated as a literal value.
            let triple_tokens = match value {
                Value::Var(ref var_ident) => {
                    quote! {
                        {
                            #[allow(unused_imports)] use ::tribles::query::TriblePattern;
                            let v_var = { #af_ident.as_variable(#var_ident) };
                            constraints.push(Box::new(#set_ident.pattern(#e_ident, #a_var_ident, v_var)));
                        }
                    }
                }
                Value::Expr(ref expr) => {
                    quote! {
                        {
                            #[allow(unused_imports)] use ::tribles::query::TriblePattern;
                            let #v_tmp_ident = #af_ident.value_from(#expr);
                            let v_var = #af_ident.as_variable(#ctx_ident.next_variable());
                            constraints.push(Box::new(v_var.is(#v_tmp_ident)));
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
            let mut constraints: ::std::vec::Vec<Box<dyn ::tribles::query::Constraint>> = ::std::vec::Vec::new();
            let #ctx_ident = __local_find_context!();
            let #set_ident = #set;
            #attr_tokens
            #entity_tokens
            ::tribles::query::intersectionconstraint::IntersectionConstraint::new(constraints)
        }
    };

    Ok(output.into())
}

/// Procedural implementation of the `entity!` macro.
///
/// Simplified form: entity!{ [id_spec @] <field>: <value>, ... }
/// Examples:
///   entity!{ metadata::name: "x" }
///   entity!{ ( &my_id ) @ metadata::name: "x" }
///   entity!{ my_id @ metadata::name: "x" }
#[proc_macro]
pub fn entity(input: TokenStream) -> TokenStream {
    match entity_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

fn entity_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let input: proc_macro2::TokenStream = input.into();
    let wrapped = quote! { { #input } };

    // Parse simplified invocation: optional id expr + attributes
    let Entity { id, attributes } = syn::parse2(wrapped)?;

    let set_init = quote! { let mut set = ::tribles::trible::TribleSet::new(); };

    // Caller must supply an expression that evaluates to a `&ExclusiveId`.
    let id_init: TokenStream2 = if let Some(val) = id {
        match val {
            Value::Expr(expr) => quote! { let id_ref: &::tribles::id::ExclusiveId = #expr; },
            Value::Var(ident) => {
                return Err(syn::Error::new_spanned(
                    ident,
                    "variable bindings (?ident) are not allowed in entity!; use a literal expression here",
                ));
            }
        }
    } else {
        quote! {
            let id_tmp: ::tribles::id::ExclusiveId = ::tribles::id::rngid();
            let id_ref: &::tribles::id::ExclusiveId = &id_tmp;
        }
    };

    let mut insert_tokens = TokenStream2::new();
    for (i, attr) in attributes.into_iter().enumerate() {
        let field_expr = attr.name;
        let value_expr = match attr.value {
            Value::Expr(e) => e,
            Value::Var(id) => {
                return Err(syn::Error::new_spanned(
                    id,
                    "variable bindings (?ident) are not allowed in entity!; use a literal expression here",
                ));
            }
        };
        let af_ident = format_ident!("__af{}", i, span = Span::mixed_site());
        let val_ident = format_ident!("__val{}", i, span = Span::mixed_site());
        let stmt = quote! {
            {
                let #af_ident = #field_expr;
                let #val_ident = #af_ident.value_from(#value_expr);
                let __a_id = #af_ident.id();
                set.insert(&::tribles::trible::Trible::new(id_ref, &__a_id, &#val_ident));
            }
        };
        insert_tokens.extend(stmt);
    }

    let output = quote! {
        {
            #set_init
            #id_init
            #insert_tokens
            set
        }
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

/// Parsed input for the [`pattern_changes`] macro.
///
/// The new simplified invocation takes the form `current, changes, [ .. ]`.
/// Both dataset expressions evaluate to [`TribleSet`]s. `current` is the full
/// dataset and `changes` represents the newly inserted tribles. The pattern
/// syntax matches that of [`pattern!`].
struct PatternChangesInput {
    curr: Expr,
    changes: Expr,
    pattern: Vec<Entity>,
}

impl Parse for PatternChangesInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
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
            curr,
            changes,
            pattern,
        })
    }
}

fn pattern_changes_impl(input: TokenStream) -> syn::Result<TokenStream> {
    use std::collections::HashMap;

    let PatternChangesInput {
        curr,
        changes,
        pattern,
    } = syn::parse(input)?;
    // We always generate expansions referencing the canonical ::tribles crate
    // path; no legacy crate/ns parameters are accepted by this macro.
    let crate_path_ts: TokenStream2 = quote! { ::tribles };
    let _crate_path = crate_path_ts.clone();

    // Identifiers used throughout the expansion
    let ctx_ident = format_ident!("__ctx", span = Span::mixed_site());
    let curr_ident = format_ident!("__curr", span = Span::mixed_site());
    let delta_ident = format_ident!("__delta", span = Span::mixed_site());

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

    // Map from tokenized attribute expression -> (attribute_var_ident, attr_field_ident)
    let mut attr_map: HashMap<String, (Ident, Ident)> = HashMap::new();
    let mut attr_idx = 0usize;
    let mut value_idx = 0usize;

    for (entity_idx, entity) in pattern.into_iter().enumerate() {
        let e_ident = format_ident!("__e{}", entity_idx, span = Span::mixed_site());
        match entity.id {
            Some(ref id_val) => match id_val {
                Value::Var(ref ident) => {
                    entity_decl_tokens.extend(quote! { let #e_ident = #ident; });
                }
                Value::Expr(ref id_expr) => {
                    entity_const_tokens.extend(quote! {
                            let #e_ident: ::tribles::query::Variable<::tribles::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                            constraints.push(Box::new(#e_ident.is(::tribles::value::ToValue::to_value(#id_expr))));
                        });
                }
            },
            None => {
                entity_decl_tokens.extend(quote! {
                    let #e_ident: ::tribles::query::Variable<::tribles::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                });
            }
        }

        for Attribute {
            name: attr_expr,
            value,
        } in entity.attributes
        {
            let key = attr_expr.to_token_stream().to_string();
            let (a_ident, af_ident) = attr_map
                .entry(key)
                .or_insert_with(|| {
                    let a_ident = format_ident!("__a{}", attr_idx, span = Span::mixed_site());
                    let af_ident = format_ident!("__af{}", attr_idx, span = Span::mixed_site());
                    attr_idx += 1;
                    attr_decl_tokens.extend(quote! {
                        let #af_ident = #attr_expr;
                        let #a_ident: ::tribles::query::Variable<::tribles::value::schemas::genid::GenId> = #ctx_ident.next_variable();
                    });
                    attr_const_tokens.extend(quote! {
                        constraints.push(Box::new(#a_ident.is(::tribles::value::ToValue::to_value(#af_ident.id()))));
                    });
                    (a_ident, af_ident)
                })
                .clone();

            let v_ident = format_ident!("__v{}", value_idx, span = Span::mixed_site());
            value_idx += 1;

            // For pattern_changes we use the explicit `?ident` binding form
            // to indicate a variable. Any other expression is treated as a
            // literal value.
            match value {
                Value::Expr(expr) => {
                    let val_ident = format_ident!("__c{}", value_idx, span = Span::mixed_site());
                    value_idx += 1;
                    let _raw_ident = format_ident!("__raw{}", value_idx, span = Span::mixed_site());
                    value_decl_tokens.extend(quote! {
                        let #val_ident = #af_ident.value_from(#expr);
                        let #v_ident = #af_ident.as_variable(#ctx_ident.next_variable());
                    });
                    value_const_tokens.extend(quote! {
                        constraints.push(Box::new(#v_ident.is(#val_ident)));
                    });
                }
                Value::Var(var_ident) => {
                    value_decl_tokens.extend(quote! {
                        let #v_ident = #af_ident.as_variable(#var_ident);
                    });
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
                let mut constraints: ::std::vec::Vec<Box<dyn ::tribles::query::Constraint>> = ::std::vec::Vec::new();
                #[allow(unused_imports)] use ::tribles::query::TriblePattern;
                #triple_tokens
                ::tribles::query::intersectionconstraint::IntersectionConstraint::new(constraints)
            }
        };
        case_exprs.push(case);
    }

    let union_expr = quote! {
            ::tribles::query::unionconstraint::UnionConstraint::new(vec![
            #(Box::new(#case_exprs) as Box<dyn ::tribles::query::Constraint>),*
        ])
    };

    let ns_use = TokenStream2::new();

    let output = quote! {
        {
            let #ctx_ident = __local_find_context!();
                        let #curr_ident = #curr;
            let #delta_ident = #changes;
            #ns_use
            #attr_decl_tokens
            #entity_decl_tokens
            #value_decl_tokens
            let mut constraints: ::std::vec::Vec<Box<dyn ::tribles::query::Constraint>> = ::std::vec::Vec::new();
            #[allow(unused_imports)] use ::tribles::query::TriblePattern;
            #attr_const_tokens
            #entity_const_tokens
            #value_const_tokens
            constraints.push(Box::new(#union_expr));
            ::tribles::query::intersectionconstraint::IntersectionConstraint::new(constraints)
        }
    };

    Ok(output.into())
}
