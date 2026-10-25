//! Small syntactic helpers over `syn` types and token streams.

use std::collections::HashSet;

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{GenericArgument, PathArguments, Type};

/// If `ty` is syntactically `Option<T>` (`Option`, `std::option::Option` or
/// `core::option::Option`), returns `T`. Type aliases cannot be detected.
pub fn option_inner(ty: &Type) -> Option<&Type> {
    let path = match ty {
        Type::Group(g) => return option_inner(&g.elem),
        Type::Paren(p) => return option_inner(&p.elem),
        Type::Path(p) if p.qself.is_none() => &p.path,
        _ => return None,
    };
    let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    if !matches!(
        names.as_slice(),
        ["Option"] | ["std" | "core", "option", "Option"]
    ) {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &path.segments.last()?.arguments else {
        return None;
    };
    if args.args.len() != 1 {
        return None;
    }
    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

/// What kind of value a field holds, as far as syntax can tell. Used to
/// reject validation rules that cannot apply (`length` on a `u32`, ...).
/// `Unknown` (aliases, generics, user types) is never rejected here: rustc
/// checks those through the rule's trait bounds, with the error pointing at
/// the rule.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Int { unsigned: bool },
    Float,
    Bool,
    Char,
    Text,
    Collection,
    Unknown,
}

fn strip(ty: &Type) -> &Type {
    match ty {
        Type::Group(g) => strip(&g.elem),
        Type::Paren(p) => strip(&p.elem),
        Type::Reference(r) => strip(&r.elem),
        other => other,
    }
}

pub fn classify(ty: &Type) -> Class {
    match strip(ty) {
        Type::Slice(_) | Type::Array(_) => Class::Collection,
        Type::Path(p) if p.qself.is_none() => {
            let Some(last) = p.path.segments.last() else {
                return Class::Unknown;
            };
            match last.ident.to_string().as_str() {
                "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => Class::Int { unsigned: true },
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => Class::Int { unsigned: false },
                "f32" | "f64" => Class::Float,
                "bool" => Class::Bool,
                "char" => Class::Char,
                "String" | "str" | "Cow" => Class::Text,
                "Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "BinaryHeap" | "LinkedList"
                | "HashMap" | "BTreeMap" => Class::Collection,
                _ => Class::Unknown,
            }
        }
        _ => Class::Unknown,
    }
}

/// Element type of the collections `each(..)` supports.
pub fn element_type(ty: &Type) -> Option<&Type> {
    match strip(ty) {
        Type::Slice(s) => Some(&s.elem),
        Type::Array(a) => Some(&a.elem),
        Type::Path(p) if p.qself.is_none() => {
            let last = p.path.segments.last()?;
            if !matches!(
                last.ident.to_string().as_str(),
                "Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "BinaryHeap" | "LinkedList"
            ) {
                return None;
            }
            let PathArguments::AngleBracketed(args) = &last.arguments else {
                return None;
            };
            match args.args.first()? {
                GenericArgument::Type(t) => Some(t),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A type as compact source text, for messages.
pub fn show(ty: &Type) -> String {
    ty.to_token_stream()
        .to_string()
        .replace(" :: ", "::")
        .replace(":: ", "::")
        .replace(" < ", "<")
        .replace(" <", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
        .replace("& ", "&")
}

/// Every identifier (and `'lifetime`, keyed with its quote) in `tokens`.
pub fn idents_in(tokens: impl ToTokens) -> HashSet<String> {
    let mut out = HashSet::new();
    walk(tokens.into_token_stream(), &mut out);
    out
}

fn walk(stream: TokenStream, out: &mut HashSet<String>) {
    let mut after_quote = false;
    for tt in stream {
        match tt {
            TokenTree::Ident(i) => {
                let n = i.to_string();
                out.insert(if after_quote { format!("'{n}") } else { n });
                after_quote = false;
            }
            TokenTree::Group(g) => {
                walk(g.stream(), out);
                after_quote = false;
            }
            TokenTree::Punct(p) => after_quote = p.as_char() == '\'',
            TokenTree::Literal(_) => after_quote = false,
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn detects_option_spellings() {
        let a: Type = parse_quote!(Option<u8>);
        let b: Type = parse_quote!(::std::option::Option<u8>);
        let c: Type = parse_quote!(core::option::Option<u8>);
        for t in [&a, &b, &c] {
            assert!(option_inner(t).is_some());
        }
    }

    #[test]
    fn rejects_non_options() {
        for t in [
            parse_quote!(Vec<u8>),
            parse_quote!(my::Option<u8>),
            parse_quote!(u8),
            parse_quote!(&'static str),
        ] {
            let t: Type = t;
            assert!(option_inner(&t).is_none());
        }
    }

    #[test]
    fn strips_exactly_one_layer() {
        let t: Type = parse_quote!(Option<Option<u8>>);
        let inner = option_inner(&t).unwrap();
        assert!(option_inner(inner).is_some());
    }

    #[test]
    fn classifies_common_types() {
        let c = |t: Type| classify(&t);
        assert_eq!(c(parse_quote!(u32)), Class::Int { unsigned: true });
        assert_eq!(c(parse_quote!(i8)), Class::Int { unsigned: false });
        assert_eq!(c(parse_quote!(f64)), Class::Float);
        assert_eq!(c(parse_quote!(&'a str)), Class::Text);
        assert_eq!(c(parse_quote!(std::string::String)), Class::Text);
        assert_eq!(c(parse_quote!(Vec<String>)), Class::Collection);
        assert_eq!(c(parse_quote!([u8; 4])), Class::Collection);
        assert_eq!(c(parse_quote!(MyAlias)), Class::Unknown);
        assert_eq!(c(parse_quote!(T)), Class::Unknown);
    }

    #[test]
    fn element_types() {
        let t: Type = parse_quote!(Vec<String>);
        assert_eq!(show(element_type(&t).unwrap()), "String");
        let t: Type = parse_quote!(HashMap<String, u8>);
        assert!(element_type(&t).is_none());
        assert_eq!(show(&parse_quote!(Vec<&'a str>)), "Vec<&'a str>");
        assert_eq!(
            show(&parse_quote!(std::collections::HashMap<String, u8>)),
            "std::collections::HashMap<String, u8>"
        );
    }

    #[test]
    fn collects_lifetimes_and_idents() {
        let ids = idents_in(quote::quote!(&'a Vec<T>));
        assert!(ids.contains("'a") && ids.contains("T") && ids.contains("Vec"));
        assert!(!ids.contains("a"));
    }
}
