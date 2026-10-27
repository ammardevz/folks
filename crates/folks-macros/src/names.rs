//! Serialized (wire) field names, as serde would compute them.
//!
//! Validation errors are reported under the names API clients actually send,
//! so `rename` and `rename_all` have to be resolved at macro time.

use syn::{Expr, ExprLit, Lit, Meta, MetaList, MetaNameValue, Token, punctuated::Punctuated};

/// The `rename_all` rules serde supports.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenameRule {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

impl RenameRule {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "lowercase" => Self::Lower,
            "UPPERCASE" => Self::Upper,
            "PascalCase" => Self::Pascal,
            "camelCase" => Self::Camel,
            "snake_case" => Self::Snake,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnake,
            "kebab-case" => Self::Kebab,
            "SCREAMING-KEBAB-CASE" => Self::ScreamingKebab,
            _ => return None,
        })
    }

    /// Apply the rule to a `snake_case` field name.
    pub fn apply(self, field: &str) -> String {
        match self {
            Self::Lower | Self::Snake => field.to_owned(),
            Self::Upper | Self::ScreamingSnake => field.to_ascii_uppercase(),
            Self::Pascal => pascal(field),
            Self::Camel => {
                let p = pascal(field);
                let mut chars = p.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
                    None => p,
                }
            }
            Self::Kebab => field.replace('_', "-"),
            Self::ScreamingKebab => field.to_ascii_uppercase().replace('_', "-"),
        }
    }
}

fn pascal(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut upper_next = true;
    for c in field.chars() {
        if c == '_' {
            upper_next = true;
        } else if upper_next {
            out.push(c.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn lit_str(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => Some(s.value()),
        _ => None,
    }
}

/// The value serde uses when *deserializing* for `key = ".."` or
/// `key(serialize = "..", deserialize = "..")`. Requests are validated as
/// they are received, so the deserialize name is the one clients send.
fn deserialize_value(meta: &Meta) -> Option<String> {
    match meta {
        Meta::NameValue(MetaNameValue { value, .. }) => lit_str(value),
        Meta::List(MetaList { tokens, .. }) => {
            let parts = syn::parse::Parser::parse2(
                Punctuated::<MetaNameValue, Token![,]>::parse_terminated,
                tokens.clone(),
            )
            .ok()?;
            parts
                .iter()
                .find(|p| p.path.is_ident("deserialize"))
                .and_then(|p| lit_str(&p.value))
        }
        Meta::Path(_) => None,
    }
}

/// The last `rename_all` in `metas`, if any is valid.
pub fn rename_all(metas: &[Meta]) -> Option<RenameRule> {
    metas
        .iter()
        .rev()
        .filter(|m| m.path().is_ident("rename_all"))
        .find_map(|m| deserialize_value(m).and_then(|v| RenameRule::parse(&v)))
}

/// An explicit field `rename`, if present.
pub fn field_rename(metas: &[Meta]) -> Option<String> {
    metas
        .iter()
        .rev()
        .filter(|m| m.path().is_ident("rename"))
        .find_map(deserialize_value)
}

/// The name a field has on the wire.
pub fn wire_name(ident: &str, rename: Option<String>, rule: Option<RenameRule>) -> String {
    rename.unwrap_or_else(|| rule.map_or_else(|| ident.to_owned(), |r| r.apply(ident)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        let f = "created_at";
        assert_eq!(RenameRule::Camel.apply(f), "createdAt");
        assert_eq!(RenameRule::Pascal.apply(f), "CreatedAt");
        assert_eq!(RenameRule::Kebab.apply(f), "created-at");
        assert_eq!(RenameRule::ScreamingKebab.apply(f), "CREATED-AT");
        assert_eq!(RenameRule::ScreamingSnake.apply(f), "CREATED_AT");
        assert_eq!(RenameRule::Upper.apply(f), "CREATED_AT");
        assert_eq!(RenameRule::Lower.apply(f), "created_at");
        assert_eq!(RenameRule::Snake.apply(f), "created_at");
    }

    #[test]
    fn rename_beats_rename_all() {
        assert_eq!(
            wire_name("a_b", Some("x".into()), Some(RenameRule::Camel)),
            "x"
        );
        assert_eq!(wire_name("a_b", None, Some(RenameRule::Camel)), "aB");
        assert_eq!(wire_name("a_b", None, None), "a_b");
    }

    #[test]
    fn parses_metas() {
        let m: Meta = syn::parse_quote!(rename_all = "camelCase");
        assert_eq!(rename_all(&[m]), Some(RenameRule::Camel));
        let m: Meta = syn::parse_quote!(rename(serialize = "s", deserialize = "d"));
        assert_eq!(field_rename(&[m]).as_deref(), Some("d"));
        let m: Meta = syn::parse_quote!(rename = "r");
        assert_eq!(field_rename(&[m]).as_deref(), Some("r"));
    }
}
