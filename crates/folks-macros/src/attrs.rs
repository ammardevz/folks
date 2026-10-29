//! Parsing of `#[entity(...)]` and the allow-listed `#[serde(...)]` keys.

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::meta::ParseNestedMeta;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Error, Ident, LitBool, LitStr, Meta, Path, Result, Token, Type, Visibility,
    parenthesized, parse_quote, token,
};

use crate::model::{Helpers, Target};
use crate::rules::{self, Rule};

/// Accumulate errors so several problems are reported at once.
pub fn push_error(acc: &mut Option<Error>, e: Error) {
    match acc {
        Some(a) => a.combine(e),
        None => *acc = Some(e),
    }
}

const FIELD_OPTIONS: &str = "id, skip, readonly, immutable, secret, sortable, response, create, update, delete, query \
     (each also as `create(ty = Type, rename = \"..\", validate(..))`; `query(eq, range, contains)`), default, default = \"..\", clearable, clearable = false, serde_as = \"..\", validate(..), normalize(..), map(..)";

/// Options inside a per-type field flag, e.g. `create(ty = X, rename = "x")`.
#[derive(Default, Clone)]
pub struct TargetOverride {
    /// `ty = ..`: the field's type in this generated type only.
    pub ty: Option<(Span, Type)>,
    /// `rename = ".."`: the serde name in this generated type only.
    pub rename: Option<LitStr>,
    /// `query(eq)`: the equality filter (the default when no mode is given).
    pub eq: Option<Span>,
    /// `query(range)`: `<field>_from` / `<field>_to` bounds.
    pub range: Option<Span>,
    /// `query(contains)`: `<field>_contains` substring filter.
    pub contains: Option<Span>,
    /// Validation rules replacing the shared rules for this target.
    pub validate: Option<(Span, Vec<Rule>)>,
}

#[derive(Default)]
pub struct FieldAttrs {
    pub id: Option<Span>,
    pub skip: Option<Span>,
    pub readonly: Option<Span>,
    pub immutable: Option<Span>,
    pub secret: Option<Span>,
    /// `sortable`: the field is a variant of the generated sort enum.
    pub sortable: Option<Span>,
    pub targets: Vec<(Target, Span)>,
    /// Indexed by `Target::index()`.
    pub overrides: [TargetOverride; 5],
    pub default: Option<(Span, Option<LitStr>)>,
    pub clearable: Option<(Span, bool)>,
    pub serde_as: Option<Type>,
    /// Span of `validate(..)` and its parsed, not yet type-checked rules.
    pub validate: Option<(Span, Vec<Rule>)>,
    pub map_from: Option<LitStr>,
    pub map_with: Option<Path>,
    pub normalize: Vec<Ident>,
}

pub fn parse_field(attrs: &[Attribute]) -> Result<FieldAttrs> {
    let mut out = FieldAttrs::default();
    let mut errors = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("entity")) {
        let res = attr.parse_nested_meta(|meta| {
            let span = meta.path.span();
            if meta.path.is_ident("id") {
                set_once(&mut out.id, span, "id")
            } else if meta.path.is_ident("skip") {
                set_once(&mut out.skip, span, "skip")
            } else if meta.path.is_ident("readonly") {
                set_once(&mut out.readonly, span, "readonly")
            } else if meta.path.is_ident("immutable") {
                set_once(&mut out.immutable, span, "immutable")
            } else if meta.path.is_ident("secret") {
                set_once(&mut out.secret, span, "secret")
            } else if meta.path.is_ident("sortable") {
                set_once(&mut out.sortable, span, "sortable")
            } else if meta.path.is_ident("clearable") {
                if out.clearable.is_some() {
                    return Err(Error::new(span, "duplicate `clearable` option"));
                }
                let value = if meta.input.peek(Token![=]) {
                    meta.value()?.parse::<LitBool>()?.value
                } else {
                    true
                };
                out.clearable = Some((span, value));
                Ok(())
            } else if meta.path.is_ident("default") {
                if out.default.is_some() {
                    return Err(Error::new(span, "duplicate `default` option"));
                }
                let lit = if meta.input.peek(Token![=]) {
                    let lit: LitStr = meta.value()?.parse()?;
                    lit.parse::<Path>().map_err(|e| {
                        Error::new(
                            e.span(),
                            format!("`default` must be a function path, e.g. `default = \"make_id\"`: {e}"),
                        )
                    })?;
                    Some(lit)
                } else {
                    None
                };
                out.default = Some((span, lit));
                Ok(())
            } else if let Some(t) = Target::from_path(&meta.path) {
                if out.targets.iter().any(|(x, _)| *x == t) {
                    return Err(Error::new(
                        span,
                        format!("duplicate `{}` option", t.keyword()),
                    ));
                }
                out.targets.push((t, span));
                if meta.input.peek(token::Paren) {
                    let probe = meta.input.fork();
                    let inner;
                    parenthesized!(inner in probe);
                    if inner.is_empty() {
                        return Err(Error::new(
                            span,
                            format!(
                                "`{0}()` is empty; write `{0}`, `{0}(ty = Type, rename = \"..\")` or `{0}(validate(..))`",
                                t.keyword()
                            ),
                        ));
                    }
                    let slot = &mut out.overrides[t.index()];
                    meta.parse_nested_meta(|m| {
                        if m.path.is_ident("validate") {
                            if slot.validate.is_some() {
                                return Err(m.error("duplicate `validate` option"));
                            }
                            let content;
                            parenthesized!(content in m.input);
                            let tokens: TokenStream = content.parse()?;
                            if tokens.is_empty() {
                                return Err(m.error("`validate()` needs at least one rule"));
                            }
                            slot.validate = Some((m.path.span(), rules::parse_rules(tokens)?));
                            Ok(())
                        } else if m.path.is_ident("ty") {
                            if slot.ty.is_some() {
                                return Err(m.error("duplicate `ty` option"));
                            }
                            let ty: Type = m.value()?.parse()?;
                            slot.ty = Some((ty.span(), ty));
                            Ok(())
                        } else if m.path.is_ident("rename") {
                            if slot.rename.is_some() {
                                return Err(m.error("duplicate `rename` option"));
                            }
                            slot.rename = Some(m.value()?.parse()?);
                            Ok(())
                        } else if t == Target::Query
                            && (m.path.is_ident("eq")
                                || m.path.is_ident("range")
                                || m.path.is_ident("contains"))
                        {
                            let mode_span = m.path.span();
                            let mode = if m.path.is_ident("eq") {
                                &mut slot.eq
                            } else if m.path.is_ident("range") {
                                &mut slot.range
                            } else {
                                &mut slot.contains
                            };
                            set_once(mode, mode_span, &m.path.to_token_stream().to_string())
                        } else if t == Target::Query {
                            Err(m.error(
                                "unknown option for `query(..)`; expected one of: eq, range, contains, ty = Type, rename = \"..\", validate(..)",
                            ))
                        } else {
                            Err(m.error(format!(
                                "unknown option for `{}(..)`; expected `ty = Type`, `rename = \"..\"` or `validate(..)`",
                                t.keyword()
                            )))
                        }
                    })?;
                    if slot.validate.is_some()
                        && slot.ty.is_none()
                        && slot.rename.is_none()
                        && slot.eq.is_none()
                        && slot.range.is_none()
                        && slot.contains.is_none()
                    {
                        // With no other target selection, validation-only blocks
                        // keep the field's established default inclusion. If an
                        // explicit target list is present, the block is added to
                        // that list below.
                        out.targets.retain(|(target, _)| *target != t);
                    }
                }
                Ok(())
            } else if meta.path.is_ident("serde_as") {
                if out.serde_as.is_some() {
                    return Err(Error::new(span, "duplicate `serde_as` option"));
                }
                let lit: LitStr = meta.value()?.parse()?;
                out.serde_as = Some(lit.parse().map_err(|e| {
                    Error::new(
                        e.span(),
                        format!(
                            "`serde_as` must be a type, e.g. `serde_as = \"DisplayFromStr\"`: {e}"
                        ),
                    )
                })?);
                Ok(())
            } else if meta.path.is_ident("validate") {
                if out.validate.is_some() {
                    return Err(Error::new(span, "duplicate `validate` option"));
                }
                if meta.input.peek(Token![=]) {
                    return Err(meta.error(
                        "`validate` takes rules, e.g. `validate(length(min = 1))`; \
                         for a custom function use `validate(custom(path::to::fn))`",
                    ));
                }
                let content;
                parenthesized!(content in meta.input);
                let tokens: TokenStream = content.parse()?;
                if tokens.is_empty() {
                    return Err(Error::new(span, "`validate()` needs at least one rule"));
                }
                out.validate = Some((span, rules::parse_rules(tokens)?));
                Ok(())
            } else if meta.path.is_ident("map") {
                let mut seen_from = false;
                let mut seen_with = false;
                let mut seen_any = false;
                meta.parse_nested_meta(|m| {
                    if m.path.is_ident("from") {
                        seen_any = true;
                        if seen_from { return Err(m.error("duplicate `map(from = ..)`")); }
                        if out.map_from.is_some() { return Err(m.error("duplicate `map(from = ..)`")); }
                        seen_from = true;
                        let lit: LitStr = m.value()?.parse()?;
                        lit.parse::<Ident>().map_err(|e| Error::new(e.span(), format!("`map(from)` must name a source field: {e}")))?;
                        out.map_from = Some(lit);
                        Ok(())
                    } else if m.path.is_ident("with") {
                        seen_any = true;
                        if seen_with { return Err(m.error("duplicate `map(with = ..)`")); }
                        if out.map_with.is_some() { return Err(m.error("duplicate `map(with = ..)`")); }
                        seen_with = true;
                        let lit: LitStr = m.value()?.parse()?;
                        out.map_with = Some(lit.parse().map_err(|e| Error::new(e.span(), format!("`map(with)` must be a function path: {e}")))?);
                        Ok(())
                    } else {
                        Err(m.error("expected `from = \"field\"` or `with = \"path\"`"))
                    }
                })?;
                if !seen_any { return Err(meta.error("`map()` needs `from = \"field\"` or `with = \"path\"`")); }
                Ok(())
            } else if meta.path.is_ident("normalize") {
                if !out.normalize.is_empty() {
                    return Err(meta.error("duplicate `normalize` option"));
                }
                let mut casing = 0;
                meta.parse_nested_meta(|m| {
                    const OPS: &[&str] = &["trim", "trim_start", "trim_end", "collapse_whitespace", "lowercase", "uppercase", "camel_case", "pascal_case", "snake_case", "kebab_case", "screaming_snake_case", "screaming_kebab_case", "title_case", "sentence_case"];
                    let name = m.path.get_ident().map(ToString::to_string).ok_or_else(|| m.error("expected a normalization operation"))?;
                    if !OPS.contains(&name.as_str()) { return Err(m.error(format!("unknown normalization operation `{name}`"))); }
                    if !["trim", "trim_start", "trim_end", "collapse_whitespace"].contains(&name.as_str()) {
                        casing += 1;
                        if casing > 1 { return Err(m.error("only one casing operation may be specified")); }
                    }
                    out.normalize.push(m.path.get_ident().unwrap().clone());
                    Ok(())
                })?;
                if out.normalize.is_empty() { return Err(meta.error("`normalize()` needs at least one operation")); }
                Ok(())
            } else {
                Err(meta.error(format!(
                    "unknown `entity` field option; expected one of: {FIELD_OPTIONS}"
                )))
            }
        });
        if let Err(e) = res {
            push_error(&mut errors, e);
        }
    }
    // Validation-only blocks preserve the default target set when used alone,
    // but compose with an explicit target list by including their target.
    if !out.targets.is_empty() {
        for target in Target::ALL {
            if let Some((span, _)) = &out.overrides[target.index()].validate
                && !out.targets.iter().any(|(existing, _)| *existing == target)
            {
                out.targets.push((target, *span));
            }
        }
    }
    match errors {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

fn set_once(slot: &mut Option<Span>, span: Span, name: &str) -> Result<()> {
    if slot.is_some() {
        return Err(Error::new(span, format!("duplicate `{name}` option")));
    }
    *slot = Some(span);
    Ok(())
}

/// Settings for one generated type (or, as `shared`, for all of them).
#[derive(Default)]
pub struct TypeConfig {
    pub name: Option<Ident>,
    pub vis: Option<Visibility>,
    pub derives: Vec<Path>,
    pub serde: Vec<Meta>,
    pub strict: Option<bool>,
}

#[derive(Default)]
pub struct ContainerAttrs {
    /// `name = ".."`: prefix of the generated type names.
    pub prefix: Option<Ident>,
    pub shared: TypeConfig,
    /// Indexed by `Target::index()`; overrides `shared`.
    pub per_type: [TypeConfig; 5],
    pub helpers: Helpers,
    /// Source model types converted into the generated response type.
    pub from_types: Vec<Type>,
    /// Merge-patch mode: every `Option<T>` field is clearable by default.
    pub patch: bool,
    /// Opt-in: `Diff` for the update request.
    pub diff: bool,
    /// Opt-in: `Filter` for the query request.
    pub matches: bool,
    /// `query(paginate)`: `page` / `per_page` in the query request.
    pub paginate: bool,
    pub schemars: bool,
    pub utoipa: bool,
    /// `rename_all` copied from the source struct's own `#[serde(..)]`.
    pub source_serde: Vec<Meta>,
}

const CONTAINER_OPTIONS: &str = "name = \"..\", vis = \"..\", derive(..), serde(..), rename_all = \"..\", \
     strict, patch, diff, matches, schemars, utoipa, no(..), and per-type blocks \
     response(..) / create(..) / update(..) / delete(..) / query(..), from(Type)";
const TYPE_OPTIONS: &str = "name = \"..\", vis = \"..\", derive(..), serde(..), rename_all = \"..\", strict \
     (and `paginate` in `query(..)`)";

/// Options valid both on the struct (all types) and inside a per-type block.
fn parse_common(meta: &ParseNestedMeta<'_>, cfg: &mut TypeConfig) -> Result<bool> {
    let p = &meta.path;
    if p.is_ident("vis") {
        if cfg.vis.is_some() {
            return Err(meta.error("duplicate `vis` option"));
        }
        let lit: LitStr = meta.value()?.parse()?;
        cfg.vis = Some(lit.parse().map_err(|e| {
            Error::new(
                e.span(),
                format!("`vis` must be a visibility such as \"pub(crate)\": {e}"),
            )
        })?);
    } else if p.is_ident("derive") {
        let content;
        parenthesized!(content in meta.input);
        cfg.derives
            .extend(Punctuated::<Path, Token![,]>::parse_terminated(&content)?);
    } else if p.is_ident("serde") {
        let content;
        parenthesized!(content in meta.input);
        for m in Punctuated::<Meta, Token![,]>::parse_terminated(&content)? {
            if m.path().is_ident("crate") {
                return Err(Error::new_spanned(
                    &m,
                    "`crate` is set by folks and cannot be overridden",
                ));
            }
            cfg.serde.push(m);
        }
    } else if p.is_ident("rename_all") {
        let lit: LitStr = meta.value()?.parse()?;
        cfg.serde.push(parse_quote!(rename_all = #lit));
    } else if p.is_ident("strict") {
        if cfg.strict.is_some() {
            return Err(meta.error("duplicate `strict` option"));
        }
        cfg.strict = Some(if meta.input.peek(Token![=]) {
            meta.value()?.parse::<LitBool>()?.value
        } else {
            true
        });
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn ident_from_lit(lit: &LitStr, what: &str) -> Result<Ident> {
    lit.parse()
        .map_err(|e| Error::new(e.span(), format!("{what} must be a valid identifier: {e}")))
}

fn require_feature(enabled: bool, meta: &ParseNestedMeta<'_>, name: &str) -> Result<()> {
    if enabled {
        Ok(())
    } else {
        Err(meta.error(format!(
            "`{name}` needs the `{name}` feature of the `folks` crate (features = [\"{name}\"])"
        )))
    }
}

pub fn parse_container(attrs: &[Attribute]) -> Result<ContainerAttrs> {
    let mut out = ContainerAttrs::default();
    let mut errors = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("entity")) {
        let res = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if out.prefix.is_some() {
                    return Err(meta.error("duplicate `name` option"));
                }
                let lit: LitStr = meta.value()?.parse()?;
                out.prefix = Some(ident_from_lit(&lit, "`name`")?);
                Ok(())
            } else if let Some(t) = Target::from_path(&meta.path) {
                let cfg = &mut out.per_type[t.index()];
                meta.parse_nested_meta(|m| {
                    if t == Target::Query && m.path.is_ident("paginate") {
                        out.paginate = true;
                        Ok(())
                    } else if m.path.is_ident("name") {
                        if cfg.name.is_some() {
                            return Err(m.error("duplicate `name` option"));
                        }
                        let lit: LitStr = m.value()?.parse()?;
                        cfg.name = Some(ident_from_lit(&lit, "`name`")?);
                        Ok(())
                    } else if parse_common(&m, cfg)? {
                        Ok(())
                    } else {
                        Err(m.error(format!(
                            "unknown option for `{}(..)`; expected one of: {TYPE_OPTIONS}",
                            t.keyword()
                        )))
                    }
                })
            } else if meta.path.is_ident("from") {
                let content;
                parenthesized!(content in meta.input);
                let sources = Punctuated::<Type, Token![,]>::parse_terminated(&content)?;
                if sources.is_empty() {
                    return Err(meta.error(
                        "`from(..)` needs at least one source type, e.g. `from(crate::Model)`",
                    ));
                }
                out.from_types.extend(sources);
                Ok(())
            } else if meta.path.is_ident("no") {
                meta.parse_nested_meta(|m| {
                    if m.path.is_ident("from") {
                        out.helpers.from = false;
                    } else if m.path.is_ident("setters") {
                        out.helpers.setters = false;
                    } else if m.path.is_ident("patch") {
                        out.helpers.patch = false;
                    } else if m.path.is_ident("default") {
                        out.helpers.default = false;
                    } else if m.path.is_ident("new") {
                        out.helpers.new = false;
                    } else {
                        return Err(m.error("expected one of: from, setters, patch, default, new"));
                    }
                    Ok(())
                })
            } else if meta.path.is_ident("patch") {
                out.patch = true;
                Ok(())
            } else if meta.path.is_ident("diff") {
                out.diff = true;
                Ok(())
            } else if meta.path.is_ident("matches") {
                out.matches = true;
                Ok(())
            } else if meta.path.is_ident("schemars") {
                require_feature(cfg!(feature = "schemars"), &meta, "schemars")?;
                out.schemars = true;
                Ok(())
            } else if meta.path.is_ident("utoipa") {
                require_feature(cfg!(feature = "utoipa"), &meta, "utoipa")?;
                out.utoipa = true;
                Ok(())
            } else if parse_common(&meta, &mut out.shared)? {
                Ok(())
            } else {
                Err(meta.error(format!(
                    "unknown `entity` option on the struct; expected one of: {CONTAINER_OPTIONS} \
                     (field options such as `id` or `skip` go on fields)"
                )))
            }
        });
        if let Err(e) = res {
            push_error(&mut errors, e);
        }
    }
    if let Some(e) = errors {
        return Err(e);
    }
    out.source_serde = serde_metas(attrs)
        .into_iter()
        .filter(|m| key(m).is_some_and(|k| k == "rename_all"))
        .collect();
    Ok(out)
}

/// Field `#[serde(..)]` keys, split by how they may be propagated.
///
/// * `always`: name-level keys that stay valid whatever the generated field's
///   type is (`rename`, `alias`) and go to every generated type.
/// * `typed`: keys that depend on the field's exact type (`with`, `default`,
///   ...). They go only to types where the field keeps its declared type
///   (response, create, delete), never to update/query fields, which are
///   `Option`-wrapped and get their own `default`/`skip_serializing_if`.
///
/// Every other key is ignored: it concerns the source struct only.
#[derive(Default)]
pub struct FieldSerde {
    pub always: Vec<Meta>,
    pub typed: Vec<Meta>,
}

const ALWAYS: &[&str] = &["rename", "alias"];
const TYPED: &[&str] = &[
    "default",
    "with",
    "serialize_with",
    "deserialize_with",
    "skip_serializing_if",
];

pub fn parse_field_serde(attrs: &[Attribute]) -> FieldSerde {
    let mut out = FieldSerde::default();
    for m in serde_metas(attrs) {
        match key(&m) {
            Some(k) if ALWAYS.contains(&k.as_str()) => out.always.push(m),
            Some(k) if TYPED.contains(&k.as_str()) => out.typed.push(m),
            _ => {}
        }
    }
    out
}

/// Parse every `#[serde(..)]` attribute. Attributes that do not parse as a
/// meta list are skipped: serde's own derive reports those.
fn serde_metas(attrs: &[Attribute]) -> Vec<Meta> {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("serde"))
        .filter_map(|a| {
            a.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .collect()
}

fn key(m: &Meta) -> Option<String> {
    m.path().get_ident().map(ToString::to_string)
}
