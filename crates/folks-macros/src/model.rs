//! The parsed, validated description of an entity.

use proc_macro2::Span;
use syn::ext::IdentExt;
use syn::{
    Attribute, Data, DeriveInput, Error, Fields, Generics, Ident, LitStr, Meta, Path, Result, Type,
    Visibility, parse_quote,
};

use crate::attrs::{self, ContainerAttrs, FieldAttrs, push_error};
use crate::rules::{self, Ctx, Rule};
use crate::types::option_inner;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Response,
    Create,
    Update,
    Delete,
    Query,
}

impl Target {
    pub const ALL: [Target; 5] = [
        Target::Response,
        Target::Create,
        Target::Update,
        Target::Delete,
        Target::Query,
    ];

    /// Attribute keyword, e.g. `update`.
    pub fn keyword(self) -> &'static str {
        match self {
            Target::Response => "response",
            Target::Create => "create",
            Target::Update => "update",
            Target::Delete => "delete",
            Target::Query => "query",
        }
    }

    /// Suffix of the generated type name, e.g. `UpdateRequest`.
    pub fn suffix(self) -> &'static str {
        match self {
            Target::Response => "Response",
            Target::Create => "CreateRequest",
            Target::Update => "UpdateRequest",
            Target::Delete => "DeleteRequest",
            Target::Query => "QueryRequest",
        }
    }

    pub fn from_path(p: &Path) -> Option<Target> {
        Target::ALL.into_iter().find(|t| p.is_ident(t.keyword()))
    }

    /// Update and query fields are always `Option`-wrapped.
    pub fn wraps_in_option(self) -> bool {
        matches!(self, Target::Update | Target::Query)
    }

    pub fn index(self) -> usize {
        self as usize
    }

    fn bit(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Clone, Copy, Default)]
pub struct TargetSet(u8);

impl TargetSet {
    fn with(mut self, t: Target) -> Self {
        self.0 |= t.bit();
        self
    }

    pub fn contains(self, t: Target) -> bool {
        self.0 & t.bit() != 0
    }
}

/// Which generated helpers are emitted (`#[entity(no(..))]` turns them off).
#[derive(Clone, Copy)]
pub struct Helpers {
    pub from: bool,
    pub setters: bool,
    pub patch: bool,
    pub default: bool,
    pub new: bool,
}

impl Default for Helpers {
    fn default() -> Self {
        Self {
            from: true,
            setters: true,
            patch: true,
            default: true,
            new: true,
        }
    }
}

pub struct Field<'a> {
    pub ident: &'a Ident,
    pub ty: &'a Type,
    pub docs: Vec<&'a Attribute>,
    pub is_id: bool,
    pub targets: TargetSet,
    pub serde_always: Vec<Meta>,
    pub serde_typed: Vec<Meta>,
    pub serde_as: Option<Type>,
    /// `#[entity(secret)]`: redacted in `Debug`, never in response or query.
    pub secret: bool,
    /// Type-checked `validate(..)` rules (empty if none).
    pub rules: Vec<Rule>,
    /// `#[entity(default)]` (`Some(None)`) or `default = "path"` for create requests.
    pub default: Option<Option<LitStr>>,
    /// Update requests can set this `Option<T>` field to null.
    pub clearable: bool,
}

impl Field<'_> {
    /// Is the generated field a three-state `Maybe<T>` (a clearable update field)?
    pub fn is_maybe(&self, target: Target) -> bool {
        self.clearable && target == Target::Update
    }

    /// Does the macro add an `Option` layer around the declared type in `target`?
    pub fn wrapped_in(&self, target: Target) -> bool {
        target.wraps_in_option()
            && (!self.optional_in_source() || (target == Target::Update && self.clearable))
    }

    /// Is the field an `Option` in the generated type for `target`?
    pub fn optional_in(&self, target: Target) -> bool {
        target.wraps_in_option() || option_inner(self.ty).is_some()
    }

    /// The declared type minus one `Option` layer.
    pub fn inner_ty(&self) -> &Type {
        option_inner(self.ty).unwrap_or(self.ty)
    }

    /// Is the declared type itself an `Option`?
    pub fn optional_in_source(&self) -> bool {
        option_inner(self.ty).is_some()
    }
}

pub struct Entity<'a> {
    pub ident: &'a Ident,
    /// The entity's own visibility; generated types inherit it unless overridden.
    pub entity_vis: &'a Visibility,
    pub generics: &'a Generics,
    /// Prefix of the generated type names (`User` for `UserResponse`).
    pub base: String,
    pub cfg: ContainerAttrs,
    pub fields: Vec<Field<'a>>,
    pub id: usize,
}

impl<'a> Entity<'a> {
    pub fn parse(input: &'a DeriveInput) -> Result<Self> {
        let Data::Struct(data) = &input.data else {
            return Err(Error::new_spanned(
                &input.ident,
                "`Entity` can only be derived for structs with named fields",
            ));
        };
        let Fields::Named(named) = &data.fields else {
            return Err(Error::new_spanned(
                &input.ident,
                "`Entity` can only be derived for structs with named fields",
            ));
        };

        let container = attrs::parse_container(&input.attrs)?;
        let base = container
            .prefix
            .as_ref()
            .unwrap_or(&input.ident)
            .unraw()
            .to_string();

        let mut errors = None;
        let mut fields = Vec::new();
        let mut id_spans = Vec::new();
        for f in &named.named {
            match Field::parse(f, &input.ident, container.patch) {
                Ok((field, id_span)) => {
                    if let Some(span) = id_span {
                        id_spans.push(span);
                    }
                    fields.push(field);
                }
                Err(e) => push_error(&mut errors, e),
            }
        }

        match id_spans.as_slice() {
            // A field that failed to parse may have been the id; report only
            // the real problem in that case.
            [] if errors.is_some() => {}
            [] => push_error(
                &mut errors,
                Error::new_spanned(
                    &input.ident,
                    "`Entity` requires exactly one field marked `#[entity(id)]`",
                ),
            ),
            [_] => {}
            [first, rest @ ..] => {
                for span in rest {
                    push_error(
                        &mut errors,
                        Error::new(*span, "only one field may be marked `#[entity(id)]`"),
                    );
                }
                push_error(
                    &mut errors,
                    Error::new(*first, "first `#[entity(id)]` field is here"),
                );
            }
        }
        if let Some(e) = errors {
            return Err(e);
        }

        let entity = Entity {
            ident: &input.ident,
            entity_vis: &input.vis,
            generics: &input.generics,
            base,
            cfg: container,
            fields,
            id: 0,
        };
        entity.check_type_names()?;
        let mut entity = entity;

        entity.id = entity.fields.iter().position(|f| f.is_id).ok_or_else(|| {
            Error::new_spanned(&input.ident, "internal error: id field not found")
        })?;
        Ok(entity)
    }

    /// Two generated types must not share a name (possible via `names(..)`).
    fn check_type_names(&self) -> Result<()> {
        let mut seen: Vec<(String, Target)> = Vec::new();
        for t in Target::ALL {
            let name = self.type_name(t).to_string();
            if let Some((_, other)) = seen.iter().find(|(n, _)| *n == name) {
                return Err(Error::new(
                    self.type_name(t).span(),
                    format!(
                        "the {} and {} types would both be named `{name}`; \
                         give one a different name, e.g. `response(name = \"..\")`",
                        other.keyword(),
                        t.keyword()
                    ),
                ));
            }
            seen.push((name, t));
        }
        Ok(())
    }

    pub fn id_field(&self) -> &Field<'a> {
        &self.fields[self.id]
    }

    pub fn type_name(&self, target: Target) -> Ident {
        self.cfg.per_type[target.index()]
            .name
            .clone()
            .unwrap_or_else(|| {
                Ident::new(
                    &format!("{}{}", self.base, target.suffix()),
                    self.ident.span(),
                )
            })
    }

    /// Visibility of a generated type: per-type, else shared, else the entity's.
    pub fn vis(&self, target: Target) -> &Visibility {
        self.cfg.per_type[target.index()]
            .vis
            .as_ref()
            .or(self.cfg.shared.vis.as_ref())
            .unwrap_or(self.entity_vis)
    }

    /// Shared derives first, then the type's own.
    pub fn derives(&self, target: Target) -> Vec<&Path> {
        self.cfg
            .shared
            .derives
            .iter()
            .chain(&self.cfg.per_type[target.index()].derives)
            .collect()
    }

    /// Container `#[serde(..)]` keys for a generated type: copied source
    /// `rename_all`, shared, per-type, then `strict`. `strict` (set on the
    /// struct) covers the request types, not the response; a type's own
    /// `strict`/`strict = false` overrides it.
    pub fn serde_attrs(&self, target: Target) -> Vec<Meta> {
        let own = &self.cfg.per_type[target.index()];
        let own_serde = self.cfg.shared.serde.iter().chain(&own.serde);
        // `rename_all` on the entity attribute replaces the one copied from
        // the source struct (serde rejects duplicates).
        let overridden = own_serde.clone().any(|m| m.path().is_ident("rename_all"));
        let mut out: Vec<Meta> = self
            .cfg
            .source_serde
            .iter()
            .filter(|_| !overridden)
            .cloned()
            .collect();
        out.extend(own_serde.cloned());
        let strict = own
            .strict
            .or(self
                .cfg
                .shared
                .strict
                .filter(|_| target != Target::Response))
            .unwrap_or(false);
        if strict {
            out.push(parse_quote!(deny_unknown_fields));
        }
        out
    }

    pub fn fields_for(&self, target: Target) -> Vec<&Field<'a>> {
        self.fields
            .iter()
            .filter(|f| f.targets.contains(target))
            .collect()
    }
}

impl<'a> Field<'a> {
    fn parse(f: &'a syn::Field, entity: &Ident, patch: bool) -> Result<(Self, Option<Span>)> {
        let ident = f
            .ident
            .as_ref()
            .ok_or_else(|| Error::new_spanned(f, "`Entity` requires named fields"))?;
        let attrs = attrs::parse_field(&f.attrs)?;
        let targets = resolve_targets(&attrs, entity, ident)?;
        let serde = attrs::parse_field_serde(&f.attrs);

        if let Some((span, _)) = &attrs.default
            && !targets.contains(Target::Create)
        {
            return Err(Error::new(
                *span,
                "`default` fills omitted fields of create requests, but this field is not in them",
            ));
        }
        let rules = match attrs.validate {
            None => Vec::new(),
            Some((span, rules)) => {
                let ctx = Ctx {
                    in_create: targets.contains(Target::Create),
                    in_any_request: [
                        Target::Create,
                        Target::Update,
                        Target::Delete,
                        Target::Query,
                    ]
                    .into_iter()
                    .any(|t| targets.contains(t)),
                };
                rules::check(&rules, &f.ty, &ctx, span)?;
                rules
            }
        };

        // Explicit `clearable` is validated; the struct-level `patch` mode only
        // applies where it can (an `Option<T>` field in update requests
        // without `serde_as`).
        let clearable = match attrs.clearable {
            Some((span, true)) => {
                if !targets.contains(Target::Update) {
                    return Err(Error::new(
                        span,
                        "`clearable` needs the field in update requests (it is skipped or excluded)",
                    ));
                }
                if option_inner(&f.ty).is_none() {
                    return Err(Error::new(
                        span,
                        "`clearable` needs a field declared `Option<T>`: only nullable fields can be set to null",
                    ));
                }
                if attrs.serde_as.is_some() {
                    return Err(Error::new(
                        span,
                        "`clearable` cannot be combined with `serde_as`",
                    ));
                }
                true
            }
            Some((_, false)) => false,
            None => {
                patch
                    && targets.contains(Target::Update)
                    && option_inner(&f.ty).is_some()
                    && attrs.serde_as.is_none()
            }
        };

        let field = Field {
            ident,
            ty: &f.ty,
            docs: f
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("doc"))
                .collect(),
            is_id: attrs.id.is_some(),
            targets,
            serde_always: serde.always,
            serde_typed: serde.typed,
            serde_as: attrs.serde_as,
            secret: attrs.secret.is_some(),
            rules,
            default: attrs.default.map(|(_, path)| path),
            clearable,
        };
        Ok((field, attrs.id))
    }
}

/// Inclusion rules.
///
/// | field    | default                         | explicit list               |
/// |----------|---------------------------------|-----------------------------|
/// | ordinary | response, create, update, query | exactly the listed targets  |
/// | id       | response, query, delete         | `response`/`query` only; delete always |
/// | `skip`   | none                            | conflicts with any list     |
/// | `readonly`  | response, query              | shorthand, no list          |
/// | `immutable` | response, create, query      | shorthand, no list          |
/// | `secret` | create, update                  | list without response/query |
fn resolve_targets(attrs: &FieldAttrs, entity: &Ident, field: &Ident) -> Result<TargetSet> {
    let is_id = attrs.id.is_some();

    if let Some(secret) = attrs.secret {
        let at = format!("`{entity}::{field}`");
        if is_id {
            return Err(Error::new(
                secret,
                format!(
                    "{at}: `secret` cannot be used on the `#[entity(id)]` field, which is always in responses and queries; remove `secret`"
                ),
            ));
        }
        if let Some(skip) = attrs.skip {
            return Err(Error::new(
                skip,
                format!(
                    "{at}: `secret` has no effect on a skipped field; remove `secret` or `skip`"
                ),
            ));
        }
        for (keyword, span) in [("readonly", attrs.readonly), ("immutable", attrs.immutable)] {
            if let Some(span) = span {
                return Err(Error::new(
                    span,
                    format!(
                        "{at}: `{keyword}` puts the field in responses and queries, which a `secret` field must never be in; \
                         remove `{keyword}` and use `#[entity(secret, create, update)]`"
                    ),
                ));
            }
        }
        for (t, span) in &attrs.targets {
            if matches!(t, Target::Response | Target::Query) {
                return Err(Error::new(
                    *span,
                    format!(
                        "{at}: a `secret` field cannot be in {} types, because secrets are never returned or used as query filters; \
                         remove `{}` (for example `#[entity(secret, create, update)]`)",
                        t.keyword(),
                        t.keyword()
                    ),
                ));
            }
        }
        if attrs.targets.is_empty() {
            // A bare `secret` means: accepted on input, never returned.
            return Ok(TargetSet::default()
                .with(Target::Create)
                .with(Target::Update));
        }
    }

    if let Some(skip) = attrs.skip {
        if is_id {
            return Err(Error::new(
                skip,
                "the `#[entity(id)]` field cannot be skipped",
            ));
        }
        if let Some((t, span)) = attrs.targets.first() {
            return Err(Error::new(
                *span,
                format!("`{}` conflicts with `skip` on field `{field}`", t.keyword()),
            ));
        }
        if let Some(span) = attrs.readonly.or(attrs.immutable) {
            return Err(Error::new(
                span,
                "`readonly`/`immutable` conflict with `skip`",
            ));
        }
        return Ok(TargetSet::default());
    }

    if let Some(span) = attrs.readonly.or(attrs.immutable) {
        let keyword = if attrs.readonly.is_some() {
            "readonly"
        } else {
            "immutable"
        };
        if is_id {
            return Err(Error::new(
                span,
                format!("the id is already read-only; remove `{keyword}`"),
            ));
        }
        if attrs.readonly.is_some() && attrs.immutable.is_some() {
            return Err(Error::new(span, "`readonly` conflicts with `immutable`"));
        }
        if let Some((t, span)) = attrs.targets.first() {
            return Err(Error::new(
                *span,
                format!(
                    "`{}` conflicts with `{keyword}` on field `{field}`",
                    t.keyword()
                ),
            ));
        }
        let mut set = TargetSet::default()
            .with(Target::Response)
            .with(Target::Query);
        if attrs.immutable.is_some() {
            set = set.with(Target::Create);
        }
        return Ok(set);
    }

    if is_id {
        for (t, span) in &attrs.targets {
            match t {
                Target::Create | Target::Update => {
                    return Err(Error::new(
                        *span,
                        format!(
                            "the id field cannot be included in the {} request",
                            t.keyword()
                        ),
                    ));
                }
                Target::Delete => {
                    return Err(Error::new(
                        *span,
                        "the id field is always part of the delete request; remove `delete`",
                    ));
                }
                Target::Response | Target::Query => {}
            }
        }
        let mut set = TargetSet::default().with(Target::Delete);
        if attrs.targets.is_empty() {
            set = set.with(Target::Response).with(Target::Query);
        }
        for (t, _) in &attrs.targets {
            set = set.with(*t);
        }
        return Ok(set);
    }

    if attrs.targets.is_empty() {
        return Ok([
            Target::Response,
            Target::Create,
            Target::Update,
            Target::Query,
        ]
        .into_iter()
        .fold(TargetSet::default(), TargetSet::with));
    }
    Ok(attrs
        .targets
        .iter()
        .fold(TargetSet::default(), |s, (t, _)| s.with(*t)))
}
