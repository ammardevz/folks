//! The typed `validate(..)` rule language: parsing, static checks against the
//! field type, and code generation.
//!
//! Rules run garde's rule functions (`folks::garde::rules::*::apply`), so there
//! is no `::garde` path or version for the consumer to get right. Generated
//! code is spanned at the rule's own tokens, so a trait-bound failure points at
//! the offending rule rather than at the derive.

use std::collections::HashSet;

use proc_macro2::{Literal, Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Error, Expr, ExprLit, ExprUnary, Ident, Lit, LitStr, Path, Result, Token, Type, UnOp,
    parenthesized, token,
};

use crate::attrs::push_error;
use crate::types::{Class, classify, element_type, option_inner, show};

pub struct Rule {
    pub span: Span,
    pub kind: Kind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LengthMode {
    Simple,
    Bytes,
    Chars,
    Utf16,
}

pub struct Number {
    pub span: Span,
    pub tokens: TokenStream,
    pub value: f64,
    pub float: bool,
    pub negative: bool,
}

pub enum Kind {
    Length {
        mode: LengthMode,
        min: Option<u64>,
        max: Option<u64>,
    },
    Range {
        min: Option<Number>,
        max: Option<Number>,
    },
    Ascii,
    Alphanumeric,
    Email,
    Url,
    Contains(LitStr),
    Prefix(LitStr),
    Suffix(LitStr),
    Pattern(LitStr),
    Required,
    Custom(Path),
    Each(Vec<Rule>),
}

impl Kind {
    fn name(&self) -> &'static str {
        match self {
            Kind::Length { .. } => "length",
            Kind::Range { .. } => "range",
            Kind::Ascii => "ascii",
            Kind::Alphanumeric => "alphanumeric",
            Kind::Email => "email",
            Kind::Url => "url",
            Kind::Contains(_) => "contains",
            Kind::Prefix(_) => "prefix",
            Kind::Suffix(_) => "suffix",
            Kind::Pattern(_) => "pattern",
            Kind::Required => "required",
            Kind::Custom(_) => "custom",
            Kind::Each(_) => "each",
        }
    }
}

const RULE_NAMES: &[&str] = &[
    "length",
    "range",
    "ascii",
    "alphanumeric",
    "email",
    "url",
    "contains",
    "prefix",
    "suffix",
    "pattern",
    "required",
    "custom",
    "each",
];

// ------------------------------------------------------------------ parsing

pub fn parse_rules(tokens: TokenStream) -> Result<Vec<Rule>> {
    let rules = Punctuated::<Rule, Token![,]>::parse_terminated.parse2(tokens)?;
    Ok(rules.into_iter().collect())
}

struct Arg {
    key: Ident,
    value: Option<Expr>,
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: Ident = input.parse()?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Arg { key, value })
    }
}

fn parse_args(input: ParseStream) -> Result<Vec<Arg>> {
    if !input.peek(token::Paren) {
        return Ok(Vec::new());
    }
    let content;
    parenthesized!(content in input);
    Ok(Punctuated::<Arg, Token![,]>::parse_terminated(&content)?
        .into_iter()
        .collect())
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let span = name.span();
        let key = name.to_string();
        let kind = match key.as_str() {
            "length" => parse_length(input, span)?,
            "range" => parse_range(input, span)?,
            "ascii" => no_args(input, &name, Kind::Ascii)?,
            "alphanumeric" => no_args(input, &name, Kind::Alphanumeric)?,
            "required" => no_args(input, &name, Kind::Required)?,
            "email" => {
                require_feature(cfg!(feature = "email"), span, "email")?;
                no_args(input, &name, Kind::Email)?
            }
            "url" => {
                require_feature(cfg!(feature = "url"), span, "url")?;
                no_args(input, &name, Kind::Url)?
            }
            "contains" => Kind::Contains(string_arg(input, &name)?),
            "prefix" => Kind::Prefix(string_arg(input, &name)?),
            "suffix" => Kind::Suffix(string_arg(input, &name)?),
            "pattern" => {
                require_feature(cfg!(feature = "pattern"), span, "pattern")?;
                let lit = string_arg(input, &name)?;
                check_regex(&lit)?;
                Kind::Pattern(lit)
            }
            "custom" => {
                let content;
                parenthesized!(content in input);
                let path: Path = content.parse()?;
                if !content.is_empty() {
                    return Err(content.error("`custom(..)` takes a single function path"));
                }
                Kind::Custom(path)
            }
            "each" => {
                let content;
                parenthesized!(content in input);
                let inner: Vec<Rule> = Punctuated::<Rule, Token![,]>::parse_terminated(&content)?
                    .into_iter()
                    .collect();
                if inner.is_empty() {
                    return Err(Error::new(span, "`each(..)` needs at least one rule"));
                }
                Kind::Each(inner)
            }
            other => return Err(unknown_rule(&name, other)),
        };
        Ok(Rule { span, kind })
    }
}

fn require_feature(enabled: bool, span: Span, name: &str) -> Result<()> {
    if enabled {
        Ok(())
    } else {
        Err(Error::new(
            span,
            format!(
                "the `{name}` rule needs the `{name}` feature of the `folks` crate (features = [\"{name}\"])"
            ),
        ))
    }
}

fn no_args(input: ParseStream, name: &Ident, kind: Kind) -> Result<Kind> {
    if input.peek(token::Paren) {
        let content;
        parenthesized!(content in input);
        if !content.is_empty() {
            return Err(Error::new(
                name.span(),
                format!("`{name}` takes no arguments"),
            ));
        }
    }
    Ok(kind)
}

fn string_arg(input: ParseStream, name: &Ident) -> Result<LitStr> {
    if !input.peek(token::Paren) {
        return Err(Error::new(
            name.span(),
            format!("`{name}` needs a string, e.g. `{name}(\"text\")`"),
        ));
    }
    let content;
    parenthesized!(content in input);
    let lit: LitStr = content.parse()?;
    if !content.is_empty() {
        return Err(content.error(format!("`{name}` takes a single string")));
    }
    Ok(lit)
}

#[cfg(feature = "pattern")]
fn check_regex(lit: &LitStr) -> Result<()> {
    regex_syntax::Parser::new()
        .parse(&lit.value())
        .map(|_| ())
        .map_err(|e| Error::new(lit.span(), format!("invalid regular expression: {e}")))
}

#[cfg(not(feature = "pattern"))]
fn check_regex(_: &LitStr) -> Result<()> {
    Ok(())
}

fn uint(arg: &Arg) -> Result<(u64, Span)> {
    let name = &arg.key;
    match &arg.value {
        Some(Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        })) => Ok((i.base10_parse::<u64>()?, i.span())),
        other => Err(Error::new(
            other.as_ref().map_or(name.span(), Spanned::span),
            format!("`{name}` must be a non-negative integer literal, e.g. `{name} = 1`"),
        )),
    }
}

fn number(expr: &Expr, what: &Ident) -> Result<Number> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => Ok(Number {
            span: i.span(),
            tokens: i.to_token_stream(),
            value: i.base10_parse::<f64>()?,
            float: false,
            negative: false,
        }),
        Expr::Lit(ExprLit {
            lit: Lit::Float(f), ..
        }) => Ok(Number {
            span: f.span(),
            tokens: f.to_token_stream(),
            value: f.base10_parse::<f64>()?,
            float: true,
            negative: false,
        }),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr: inner,
            ..
        }) => {
            let mut n = number(inner, what)?;
            let tokens = n.tokens;
            n.tokens = quote!(-#tokens);
            n.value = -n.value;
            n.negative = true;
            Ok(n)
        }
        other => Err(Error::new_spanned(
            other,
            format!("`{what}` must be a numeric literal, e.g. `{what} = 1`"),
        )),
    }
}

fn set_once<T>(slot: &mut Option<T>, arg: &Arg, value: T) -> Result<()> {
    if slot.is_some() {
        return Err(Error::new(
            arg.key.span(),
            format!("duplicate `{}` argument", arg.key),
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn parse_length(input: ParseStream, span: Span) -> Result<Kind> {
    let args = parse_args(input)?;
    let (mut mode, mut min, mut max, mut equal) = (None, None, None, None);
    for a in &args {
        match a.key.to_string().as_str() {
            "min" => set_once(&mut min, a, uint(a)?)?,
            "max" => set_once(&mut max, a, uint(a)?)?,
            "equal" => set_once(&mut equal, a, uint(a)?)?,
            m @ ("simple" | "bytes" | "chars" | "utf16") => {
                if a.value.is_some() {
                    return Err(Error::new(
                        a.key.span(),
                        format!("`{m}` is a flag and takes no value"),
                    ));
                }
                let m = match m {
                    "simple" => LengthMode::Simple,
                    "bytes" => LengthMode::Bytes,
                    "chars" => LengthMode::Chars,
                    _ => LengthMode::Utf16,
                };
                set_once(&mut mode, a, m)?;
            }
            other => {
                return Err(Error::new(
                    a.key.span(),
                    format!(
                        "unknown `length` argument `{other}`; expected `min`, `max`, `equal` or a mode (`simple`, `bytes`, `chars`, `utf16`)"
                    ),
                ));
            }
        }
    }
    if min.is_none() && max.is_none() && equal.is_none() {
        return Err(Error::new(
            span,
            "`length` needs at least one of `min`, `max`, `equal`",
        ));
    }
    if let Some((_, eq_span)) = equal
        && (min.is_some() || max.is_some())
    {
        return Err(Error::new(
            eq_span,
            "`equal` cannot be combined with `min` or `max`",
        ));
    }
    if let (Some((lo, _)), Some((hi, hi_span))) = (min, max)
        && lo > hi
    {
        return Err(Error::new(
            hi_span,
            format!("`max` ({hi}) is lower than `min` ({lo})"),
        ));
    }
    let (min, max) = match equal {
        Some((n, _)) => (Some(n), Some(n)),
        None => (min.map(|m| m.0), max.map(|m| m.0)),
    };
    Ok(Kind::Length {
        mode: mode.unwrap_or(LengthMode::Simple),
        min,
        max,
    })
}

fn parse_range(input: ParseStream, span: Span) -> Result<Kind> {
    let args = parse_args(input)?;
    let (mut min, mut max) = (None, None);
    for a in &args {
        let slot = match a.key.to_string().as_str() {
            "min" => &mut min,
            "max" => &mut max,
            other => {
                return Err(Error::new(
                    a.key.span(),
                    format!("unknown `range` argument `{other}`; expected `min` or `max`"),
                ));
            }
        };
        let Some(value) = &a.value else {
            return Err(Error::new(
                a.key.span(),
                format!("`{}` needs a value, e.g. `{} = 1`", a.key, a.key),
            ));
        };
        set_once(slot, a, number(value, &a.key)?)?;
    }
    if min.is_none() && max.is_none() {
        return Err(Error::new(
            span,
            "`range` needs at least one of `min`, `max`",
        ));
    }
    if let (Some(lo), Some(hi)) = (&min, &max)
        && lo.value > hi.value
    {
        return Err(Error::new(hi.span, "`max` is lower than `min`"));
    }
    Ok(Kind::Range { min, max })
}

fn unknown_rule(name: &Ident, given: &str) -> Error {
    let suggestion = RULE_NAMES
        .iter()
        .map(|r| (distance(given, r), *r))
        .filter(|(d, _)| *d <= 2)
        .min()
        .map(|(_, r)| format!("; did you mean `{r}`?"))
        .unwrap_or_default();
    Error::new(
        name.span(),
        format!(
            "unknown validation rule `{given}`{suggestion} (rules: {})",
            RULE_NAMES.join(", ")
        ),
    )
}

/// Levenshtein distance, for "did you mean".
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

// ------------------------------------------------------------------- checks

/// Where the field appears; decides which rules can have an effect.
pub struct Ctx {
    pub in_create: bool,
    pub in_any_request: bool,
}

pub fn check(rules: &[Rule], declared: &Type, ctx: &Ctx, span: Span) -> Result<()> {
    if !ctx.in_any_request {
        return Err(Error::new(
            span,
            "`validate(..)` has no effect: this field is in no request type (create, update, delete, query), and responses are not validated",
        ));
    }
    let declared_option = option_inner(declared).is_some();
    let value_ty = option_inner(declared).unwrap_or(declared);
    check_list(rules, value_ty, declared_option, ctx, true)
}

fn check_list(
    rules: &[Rule],
    ty: &Type,
    declared_option: bool,
    ctx: &Ctx,
    top: bool,
) -> Result<()> {
    let mut errors = None;
    let mut seen = HashSet::new();
    for rule in rules {
        if !seen.insert(rule.kind.name()) {
            push_error(
                &mut errors,
                Error::new(rule.span, format!("duplicate `{}` rule", rule.kind.name())),
            );
            continue;
        }
        if let Err(e) = check_rule(rule, ty, declared_option, ctx, top) {
            push_error(&mut errors, e);
        }
    }
    errors.map_or(Ok(()), Err)
}

fn wrong_type(rule: &Rule, ty: &Type, needs: &str) -> Error {
    Error::new(
        rule.span,
        format!(
            "`{}` cannot be applied to `{}`: it needs {needs}",
            rule.kind.name(),
            show(ty)
        ),
    )
}

fn check_rule(rule: &Rule, ty: &Type, declared_option: bool, ctx: &Ctx, top: bool) -> Result<()> {
    let class = classify(ty);
    let ok_text = matches!(class, Class::Text | Class::Unknown);
    match &rule.kind {
        Kind::Required => {
            if !top {
                return Err(Error::new(
                    rule.span,
                    "`required` cannot be used inside `each(..)`",
                ));
            }
            if !declared_option {
                return Err(Error::new(
                    rule.span,
                    "`required` needs a field declared `Option<T>`; other fields are always present",
                ));
            }
            if !ctx.in_create {
                return Err(Error::new(
                    rule.span,
                    "`required` is only enforced in create requests, but this field is not in them",
                ));
            }
            Ok(())
        }
        Kind::Length { .. } => {
            if matches!(class, Class::Text | Class::Collection | Class::Unknown) {
                Ok(())
            } else {
                Err(wrong_type(
                    rule,
                    ty,
                    "text or a collection (String, &str, Vec<_>, ...)",
                ))
            }
        }
        Kind::Range { min, max } => match class {
            Class::Int { unsigned } => {
                for n in [min, max].into_iter().flatten() {
                    if n.float {
                        return Err(Error::new(
                            n.span,
                            format!(
                                "`{}` is an integer type; use an integer bound, not a float",
                                show(ty)
                            ),
                        ));
                    }
                    if unsigned && n.negative {
                        return Err(Error::new(
                            n.span,
                            format!(
                                "`{}` is unsigned; a negative bound can never apply",
                                show(ty)
                            ),
                        ));
                    }
                }
                Ok(())
            }
            Class::Float | Class::Unknown => Ok(()),
            _ => Err(wrong_type(rule, ty, "a number (integer or float)")),
        },
        Kind::Ascii
        | Kind::Alphanumeric
        | Kind::Email
        | Kind::Url
        | Kind::Contains(_)
        | Kind::Prefix(_)
        | Kind::Suffix(_)
        | Kind::Pattern(_) => {
            if ok_text {
                Ok(())
            } else {
                Err(wrong_type(rule, ty, "text (String or &str)"))
            }
        }
        Kind::Custom(_) => Ok(()),
        Kind::Each(inner) => {
            let elem: Type = match (class, element_type(ty)) {
                (Class::Collection, Some(elem)) => elem.clone(),
                (Class::Collection, None) => {
                    return Err(Error::new(
                        rule.span,
                        format!(
                            "`each(..)` does not support `{}`; use Vec, VecDeque, HashSet, BTreeSet, a slice or an array",
                            show(ty)
                        ),
                    ));
                }
                (Class::Unknown, _) => syn::parse_quote!(_),
                _ => {
                    return Err(wrong_type(
                        rule,
                        ty,
                        "a collection (Vec, VecDeque, HashSet, BTreeSet, slice or array)",
                    ));
                }
            };
            check_list(inner, &elem, false, ctx, false)
        }
    }
}

// ------------------------------------------------------------------ codegen

fn lit_usize(n: u64) -> Literal {
    Literal::usize_suffixed(usize::try_from(n).unwrap_or(usize::MAX))
}

/// Statements that run `rules` against `value` (an expression of type `&T`),
/// pushing a `FieldError` named by `label` (an expression of type `&str`)
/// onto the in-scope `__errors` for every failure. `required` is handled by
/// the caller because it inspects the `Option` itself.
pub fn emit(
    rules: &[Rule],
    ty: &Type,
    value: &TokenStream,
    label: &TokenStream,
    krate: &TokenStream,
) -> TokenStream {
    let class = classify(ty);
    rules
        .iter()
        .filter_map(|r| emit_one(r, ty, class, value, label, krate))
        .collect()
}

fn push(span: Span, call: TokenStream, label: &TokenStream, krate: &TokenStream) -> TokenStream {
    quote_spanned! {span=>
        if let ::core::result::Result::Err(__e) = #call {
            __errors.push(#krate::core::error::FieldError::new(#label, __e));
        }
    }
}

fn emit_one(
    rule: &Rule,
    ty: &Type,
    class: Class,
    value: &TokenStream,
    label: &TokenStream,
    krate: &TokenStream,
) -> Option<TokenStream> {
    let span = rule.span;
    let rules = quote_spanned!(span=> #krate::garde::rules);
    let call = match &rule.kind {
        Kind::Required => return None,
        Kind::Length { mode, min, max } => {
            let module = match mode {
                LengthMode::Simple => quote_spanned!(span=> simple),
                LengthMode::Bytes => quote_spanned!(span=> bytes),
                LengthMode::Chars => quote_spanned!(span=> chars),
                LengthMode::Utf16 => quote_spanned!(span=> utf16),
            };
            let min = lit_usize(min.unwrap_or(0));
            let max = max.map_or_else(
                || quote!(::core::primitive::usize::MAX),
                |m| lit_usize(m).into_token_stream(),
            );
            quote_spanned!(span=> #rules::length::#module::apply(#value, (#min, #max)))
        }
        Kind::Range { min, max } => {
            let bound = |n: &Option<Number>| match n {
                None => quote!(::core::option::Option::None),
                Some(n) => {
                    // Float fields take float bounds even when written as `1`.
                    let tokens = if class == Class::Float && !n.float {
                        let lit = Literal::f64_unsuffixed(n.value.abs());
                        if n.negative {
                            quote!(-#lit)
                        } else {
                            lit.into_token_stream()
                        }
                    } else {
                        n.tokens.clone()
                    };
                    quote!(::core::option::Option::Some(#tokens))
                }
            };
            let (lo, hi) = (bound(min), bound(max));
            quote_spanned!(span=> #rules::range::apply(#value, (#lo, #hi)))
        }
        Kind::Ascii => quote_spanned!(span=> #rules::ascii::apply(#value, ())),
        Kind::Alphanumeric => quote_spanned!(span=> #rules::alphanumeric::apply(#value, ())),
        Kind::Email => quote_spanned!(span=> #rules::email::apply(#value, ())),
        Kind::Url => quote_spanned!(span=> #rules::url::apply(#value, ())),
        Kind::Contains(lit) => quote_spanned!(span=> #rules::contains::apply(#value, (#lit,))),
        Kind::Prefix(lit) => quote_spanned!(span=> #rules::prefix::apply(#value, (#lit,))),
        Kind::Suffix(lit) => quote_spanned!(span=> #rules::suffix::apply(#value, (#lit,))),
        Kind::Pattern(lit) => quote_spanned! {span=>
            {
                static __RE: ::std::sync::LazyLock<#krate::regex::Regex> =
                    ::std::sync::LazyLock::new(|| {
                        #krate::regex::Regex::new(#lit).expect("pattern checked at compile time")
                    });
                #rules::pattern::apply(#value, (&__RE,))
            }
        },
        Kind::Custom(path) => quote_spanned!(span=> #path(#value)),
        Kind::Each(inner) => {
            let elem: Type = element_type(ty)
                .cloned()
                .unwrap_or_else(|| syn::parse_quote!(_));
            let item = quote!(__item);
            let item_label = quote!(__label.as_str());
            let body = emit(inner, &elem, &item, &item_label, krate);
            return Some(quote_spanned! {span=>
                for (__i, __item) in (#value).iter().enumerate() {
                    let __label = ::std::format!("{}[{}]", #label, __i);
                    #body
                }
            });
        }
    };
    Some(push(span, call, label, krate))
}

/// `required::apply(&self.field, ())` pushed as a `FieldError`.
pub fn emit_required(
    rule: &Rule,
    access: &TokenStream,
    label: &TokenStream,
    krate: &TokenStream,
) -> TokenStream {
    let span = rule.span;
    let call = quote_spanned!(span=> #krate::garde::rules::required::apply(#access, ()));
    push(span, call, label, krate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Result<Vec<Rule>> {
        parse_rules(src.parse().unwrap())
    }

    fn err(src: &str) -> String {
        match parse(src) {
            Ok(_) => panic!("expected an error for `{src}`"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn parses_the_basic_rules() {
        let rules = parse("length(min = 1, max = 5), range(min = -1, max = 3.5), ascii, custom(a::b), each(length(max = 2))").unwrap();
        assert_eq!(rules.len(), 5);
    }

    #[test]
    fn length_argument_errors() {
        assert!(err("length").contains("at least one of"));
        assert!(err("length(min = 5, max = 1)").contains("lower than `min`"));
        assert!(err("length(equal = 3, min = 1)").contains("cannot be combined"));
        assert!(err("length(min = -1)").contains("non-negative integer literal"));
        assert!(err("length(minn = 1)").contains("unknown `length` argument"));
        assert!(err("length(min = 1, min = 2)").contains("duplicate"));
        assert!(err("length(bytes, chars, min = 1)").contains("duplicate"));
    }

    #[test]
    fn range_argument_errors() {
        assert!(err("range").contains("at least one of"));
        assert!(err("range(min = 5, max = 1)").contains("lower than `min`"));
        assert!(err("range(min = x)").contains("numeric literal"));
    }

    #[test]
    fn unknown_rules_get_suggestions() {
        assert!(err("lenght(min = 1)").contains("did you mean `length`"));
        assert!(err("zzzzzz").contains("rules:"));
    }

    #[test]
    fn argument_shape_errors() {
        assert!(err("ascii(1)").contains("takes no arguments"));
        assert!(err("contains").contains("needs a string"));
        assert!(err("contains(1)").contains("expected string literal"));
        assert!(err("each()").contains("at least one rule"));
    }

    fn check_on(src: &str, ty: Type, in_create: bool) -> Result<()> {
        let rules = parse(src).unwrap();
        check(
            &rules,
            &ty,
            &Ctx {
                in_create,
                in_any_request: true,
            },
            Span::call_site(),
        )
    }

    #[test]
    fn type_checks() {
        use syn::parse_quote as ty;
        assert!(check_on("length(min = 1)", ty!(u32), true).is_err());
        assert!(check_on("length(min = 1)", ty!(String), true).is_ok());
        assert!(check_on("length(min = 1)", ty!(Option<Vec<u8>>), true).is_ok());
        assert!(check_on("range(min = 1)", ty!(String), true).is_err());
        assert!(check_on("range(min = 1)", ty!(Option<u8>), true).is_ok());
        assert!(check_on("range(min = 1.5)", ty!(u8), true).is_err());
        assert!(check_on("range(min = -1)", ty!(u8), true).is_err());
        assert!(check_on("range(min = -1)", ty!(i8), true).is_ok());
        assert!(check_on("range(min = 1)", ty!(f64), true).is_ok());
        assert!(check_on("ascii", ty!(u8), true).is_err());
        assert!(check_on("ascii", ty!(MyAlias), true).is_ok()); // left to rustc
        assert!(check_on("required", ty!(String), true).is_err());
        assert!(check_on("required", ty!(Option<String>), true).is_ok());
        assert!(check_on("required", ty!(Option<String>), false).is_err());
        assert!(check_on("each(length(max = 2))", ty!(Vec<String>), true).is_ok());
        assert!(check_on("each(length(max = 2))", ty!(Vec<u8>), true).is_err());
        assert!(check_on("each(length(max = 2))", ty!(String), true).is_err());
        assert!(check_on("each(length(max = 2))", ty!(HashMap<String, String>), true).is_err());
        assert!(check_on("each(required)", ty!(Vec<Option<u8>>), true).is_err());
        assert!(check_on("length(min = 1), length(max = 2)", ty!(String), true).is_err());
    }

    #[test]
    fn rules_in_response_only_fields_are_rejected() {
        let rules = parse("length(min = 1)").unwrap();
        let ty: Type = syn::parse_quote!(String);
        let e = check(
            &rules,
            &ty,
            &Ctx {
                in_create: false,
                in_any_request: false,
            },
            Span::call_site(),
        );
        assert!(e.unwrap_err().to_string().contains("no effect"));
    }

    #[test]
    fn edit_distance() {
        assert_eq!(distance("length", "length"), 0);
        assert_eq!(distance("lenght", "length"), 2);
        assert_eq!(distance("a", "abc"), 2);
    }
}
