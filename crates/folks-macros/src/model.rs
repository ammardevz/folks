//! The parsed, validated description of an entity.

use proc_macro2::Span;
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Fields, Generics, Ident, LitStr, Meta, Path, Result, Type,
    Visibility, parse_quote,
};

use crate::attrs::{self, ContainerAttrs, FieldAttrs, TargetOverride, push_error};
use crate::rules::{self, Ctx, Rule};
use crate::types::{Class, classify, option_inner, show};

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
    pub fn only(t: Target) -> Self {
        Self::default().with(t)
    }

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

/// Which filter a field of the generated query request is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QueryKind {
    /// `field`: equality (an ordinary field, the default).
    Eq,
    /// `field_from`: inclusive lower bound.
    From,
    /// `field_to`: inclusive upper bound.
    To,
    /// `field_contains`: substring filter.
    Contains,
}

/// The filters `query(..)` asks for on one entity field.
#[derive(Clone, Copy)]
pub struct QueryModes {
    pub eq: bool,
    pub range: bool,
    pub contains: bool,
}

#[derive(Clone)]
pub struct Field {
    pub ident: Ident,
    pub ty: Type,
    pub docs: Vec<Attribute>,
    pub is_id: bool,
    pub targets: TargetSet,
    /// Per-type `ty` / `rename` overrides, indexed by `Target::index()`.
    pub overrides: [TargetOverride; 5],
    pub serde_always: Vec<Meta>,
    pub serde_typed: Vec<Meta>,
    pub serde_as: Option<Type>,
    /// `#[entity(secret)]`: redacted in `Debug`, never in response or query.
    pub secret: bool,
    /// `#[entity(sortable)]`: a variant of the generated sort enum.
    pub sortable: bool,
    /// Filters requested with `query(..)`; only equality unless stated.
    pub modes: QueryModes,
    /// What this field is in the generated query request.
    pub query: QueryKind,
    /// For derived query fields (`x_from`), the entity field they read.
    pub source: Option<Ident>,
    /// Type-checked `validate(..)` rules (empty if none).
    pub rules: Vec<Rule>,
    pub map_from: Option<String>,
    pub map_with: Option<Path>,
    pub normalize: Vec<String>,
    /// `#[entity(default)]` (`Some(None)`) or `default = "path"` for create requests.
    pub default: Option<Option<LitStr>>,
    /// Update requests can set this `Option<T>` field to null.
    pub clearable: bool,
}

impl Field {
    /// Is the generated field a three-state `Maybe<T>` (a clearable update field)?
    pub fn is_maybe(&self, target: Target) -> bool {
        self.clearable && target == Target::Update
    }

    /// The declared type of the field in `target`: the `ty = ..` override if
    /// there is one, else the entity's field type.
    pub fn ty_in(&self, target: Target) -> &Type {
        match &self.overrides[target.index()].ty {
            Some((_, ty)) => ty,
            None => &self.ty,
        }
    }

    /// Span of the `ty = ..` override in `target`, if any.
    pub fn ty_override(&self, target: Target) -> Option<(Span, &Type)> {
        self.overrides[target.index()]
            .ty
            .as_ref()
            .map(|(span, ty)| (*span, ty))
    }

    /// Does the macro add an `Option` layer around the declared type in `target`?
    pub fn wrapped_in(&self, target: Target) -> bool {
        target.wraps_in_option()
            && (!self.optional_in_source(target) || (target == Target::Update && self.clearable))
    }

    /// Is the field an `Option` in the generated type for `target`?
    pub fn optional_in(&self, target: Target) -> bool {
        target.wraps_in_option() || self.optional_in_source(target)
    }

    /// The declared type in `target` minus one `Option` layer.
    pub fn inner_ty(&self, target: Target) -> &Type {
        let ty = self.ty_in(target);
        option_inner(ty).unwrap_or(ty)
    }

    /// Is the declared type in `target` itself an `Option`?
    pub fn optional_in_source(&self, target: Target) -> bool {
        option_inner(self.ty_in(target)).is_some()
    }

    /// The entity field this field reads (itself, unless derived).
    pub fn entity_ident(&self) -> &Ident {
        self.source.as_ref().unwrap_or(&self.ident)
    }

    /// Is the entity's own field an `Option`?
    pub fn declared_optional(&self) -> bool {
        option_inner(&self.ty).is_some()
    }

    pub fn rules_for(&self, target: Target) -> &[Rule] {
        self.overrides[target.index()]
            .validate
            .as_ref()
            .map_or(&self.rules, |(_, rules)| rules)
    }
}

fn normalizable_type(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    if path.path.is_ident("String") {
        return true;
    }
    path.path.segments.last().is_some_and(|segment| {
        (segment.ident == "Option" || segment.ident == "Maybe")
            && matches!(&segment.arguments, syn::PathArguments::AngleBracketed(args)
                if args.args.len() == 1 && matches!(args.args.first(), Some(syn::GenericArgument::Type(Type::Path(inner))) if inner.path.is_ident("String")))
    })
}

pub struct Entity<'a> {
    pub ident: &'a Ident,
    /// The entity's own visibility; generated types inherit it unless overridden.
    pub entity_vis: &'a Visibility,
    pub generics: &'a Generics,
    /// Prefix of the generated type names (`User` for `UserResponse`).
    pub base: String,
    pub cfg: ContainerAttrs,
    pub fields: Vec<Field>,
    pub query_fields: Vec<Field>,
    pub id: usize,
}

impl<'a> Entity<'a> {
    pub fn parse(input: &'a DeriveInput) -> Result<Self> {
        let Data::Struct(data) = &input.data else {
            return Err(Error::new_spanned(
                &input.ident,
                format!(
                    "`{}`: `Entity` / `Dto` can only be derived for structs with named fields; convert the input to a named-field struct",
                    input.ident
                ),
            ));
        };
        let Fields::Named(named) = &data.fields else {
            return Err(Error::new_spanned(
                &input.ident,
                format!(
                    "`{}`: `Entity` / `Dto` can only be derived for structs with named fields; use named fields",
                    input.ident
                ),
            ));
        };

        let container = attrs::parse_container(&input.attrs).map_err(|error| {
            let mut context = Error::new(
                input.ident.span(),
                format!(
                    "`{}`: this struct's `entity` configuration is invalid; correct or remove the option shown below",
                    input.ident
                ),
            );
            context.combine(error);
            context
        })?;
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
                Err(e) => {
                    let mut context = Error::new(
                        f.ident.as_ref().map_or_else(|| f.span(), Ident::span),
                        format!(
                            "`{}::{}`: this field's configuration is invalid; correct or remove the option shown below",
                            input.ident,
                            f.ident
                                .as_ref()
                                .map_or_else(|| "<unnamed>".to_owned(), ToString::to_string)
                        ),
                    );
                    context.combine(e);
                    push_error(&mut errors, context);
                }
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
                    format!(
                        "`{}` requires exactly one id field; mark one named field with `#[entity(id)]`",
                        input.ident
                    ),
                ),
            ),
            [_] => {}
            [first, rest @ ..] => {
                for span in rest {
                    push_error(
                        &mut errors,
                        Error::new(
                            *span,
                            format!(
                                "`{}`: only one field may be marked `#[entity(id)]`; remove this marker",
                                input.ident
                            ),
                        ),
                    );
                }
                push_error(
                    &mut errors,
                    Error::new(
                        *first,
                        format!("`{}`: the first `#[entity(id)]` field is here", input.ident),
                    ),
                );
            }
        }
        if let Some(e) = errors {
            return Err(e);
        }

        if container.from_types.is_empty()
            && let Some(field) = fields
                .iter()
                .find(|f| f.map_from.is_some() || f.map_with.is_some())
        {
            return Err(Error::new(
                field.ident.span(),
                "`map(..)` is only valid with `#[entity(from(SourceType))]`",
            ));
        }

        // `diff` and `matches` compare the entity's field with the request's
        // field, which needs the same type.
        for (enabled, target, option) in [
            (container.diff, Target::Update, "diff"),
            (container.matches, Target::Query, "matches"),
        ] {
            for f in fields.iter().filter(|_| enabled) {
                if let Some((span, _)) = f.ty_override(target) {
                    return Err(Error::new(
                        span,
                        format!(
                            "`{}::{}`: `{}(ty = ..)` cannot be combined with the struct-level `{option}` option, which compares the entity's field with the request's; \
                             remove `{option}` or the `ty` override",
                            input.ident,
                            f.ident,
                            target.keyword()
                        ),
                    ));
                }
            }
        }

        let mut entity = Entity {
            ident: &input.ident,
            entity_vis: &input.vis,
            generics: &input.generics,
            base,
            cfg: container,
            fields,
            query_fields: Vec::new(),
            id: 0,
        };
        entity.check_type_names()?;
        entity.check_query_names()?;
        entity.query_fields = entity.build_query_fields();

        entity.id = entity.fields.iter().position(|f| f.is_id).ok_or_else(|| {
            Error::new_spanned(&input.ident, "internal error: id field not found")
        })?;
        Ok(entity)
    }

    /// Name of the generated sort enum (`UserSortField`).
    pub fn sort_name(&self) -> Ident {
        Ident::new(&format!("{}SortField", self.base), self.ident.span())
    }

    /// The `sortable` fields, in declaration order.
    pub fn sortable(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.sortable).collect()
    }

    /// The generated query request must not have two fields with one name.
    fn check_query_names(&self) -> Result<()> {
        let mut names: Vec<(String, String)> = Vec::new();
        let mut add = |name: String, origin: String, span: Span| -> Result<()> {
            if let Some((_, other)) = names.iter().find(|(n, _)| *n == name) {
                return Err(Error::new(
                    span,
                    format!(
                        "`{}`: the query request would have two fields named `{name}` ({other} and {origin}); \
                         rename the entity field, or leave one of them out of the query request",
                        self.ident
                    ),
                ));
            }
            names.push((name, origin));
            Ok(())
        };
        for f in self
            .fields
            .iter()
            .filter(|f| f.targets.contains(Target::Query))
        {
            let base = f.ident.unraw().to_string();
            let origin = format!("`{base}`");
            if f.modes.eq {
                add(base.clone(), origin.clone(), f.ident.span())?;
            }
            if f.modes.range {
                add(
                    format!("{base}_from"),
                    format!("`{base}` (range)"),
                    f.ident.span(),
                )?;
                add(
                    format!("{base}_to"),
                    format!("`{base}` (range)"),
                    f.ident.span(),
                )?;
            }
            if f.modes.contains {
                add(
                    format!("{base}_contains"),
                    format!("`{base}` (contains)"),
                    f.ident.span(),
                )?;
            }
        }
        let generated = [
            ("sort", !self.sortable().is_empty()),
            ("order", !self.sortable().is_empty()),
            ("page", self.cfg.paginate),
            ("per_page", self.cfg.paginate),
        ];
        for (name, enabled) in generated {
            if enabled {
                add(
                    name.to_owned(),
                    "the field generated by `sortable` / `query(paginate)`".to_owned(),
                    self.ident.span(),
                )?;
            }
        }
        Ok(())
    }

    /// Two generated types must not share a name (possible via `names(..)`).
    fn check_type_names(&self) -> Result<()> {
        let mut seen: Vec<(String, Target)> = Vec::new();
        let mut sort_variants: Vec<(String, String)> = Vec::new();
        for field in self.sortable() {
            let variant = field
                .ident
                .unraw()
                .to_string()
                .split('_')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let mut chars = part.chars();
                    chars
                        .next()
                        .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                        .unwrap_or_default()
                })
                .collect::<String>();
            if let Some((_, other)) = sort_variants.iter().find(|(name, _)| *name == variant) {
                return Err(Error::new(
                    field.ident.span(),
                    format!(
                        "`{}::{}` and `{other}` both generate the `{variant}` sort variant; rename one field",
                        self.ident, field.ident
                    ),
                ));
            }
            sort_variants.push((variant, field.ident.to_string()));
        }
        if !self.sortable().is_empty() {
            let sort = self.sort_name().to_string();
            if let Some(t) = Target::ALL.into_iter().find(|t| self.type_name(*t) == sort) {
                return Err(Error::new(
                    self.ident.span(),
                    format!(
                        "`{}`: the sort enum and the {} type would both be named `{sort}`; give the {} type a different name, e.g. `{}(name = \"..\")`",
                        self.ident,
                        t.keyword(),
                        t.keyword(),
                        t.keyword()
                    ),
                ));
            }
        }
        for t in Target::ALL {
            let name = self.type_name(t).to_string();
            if let Some((_, other)) = seen.iter().find(|(n, _)| *n == name) {
                return Err(Error::new(
                    self.type_name(t).span(),
                    format!(
                        "`{}`: the {} and {} types would both be named `{name}`; \
                         give one a different name, e.g. `response(name = \"..\")`",
                        self.ident,
                        other.keyword(),
                        t.keyword()
                    ),
                ));
            }
            seen.push((name, t));
        }
        Ok(())
    }

    pub fn id_field(&self) -> &Field {
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

    pub fn fields_for(&self, target: Target) -> Vec<&Field> {
        if target == Target::Query {
            return self.query_fields.iter().collect();
        }
        self.fields
            .iter()
            .filter(|f| f.targets.contains(target))
            .collect()
    }

    fn build_query_fields(&self) -> Vec<Field> {
        let mut fields = Vec::new();
        for source in self
            .fields
            .iter()
            .filter(|f| f.targets.contains(Target::Query))
        {
            if source.modes.eq {
                fields.push(source.clone());
            }
            for (suffix, kind, keep_rules) in [
                ("_from", QueryKind::From, true),
                ("_to", QueryKind::To, true),
                ("_contains", QueryKind::Contains, false),
            ] {
                if (kind == QueryKind::From || kind == QueryKind::To) && !source.modes.range
                    || kind == QueryKind::Contains && !source.modes.contains
                {
                    continue;
                }
                let mut field = source.clone();
                field.ident = Ident::new(
                    &format!("{}{}", source.ident.unraw(), suffix),
                    source.ident.span(),
                );
                field.ty = source.inner_ty(Target::Query).clone();
                field.rules = source.rules_for(Target::Query).to_vec();
                field.targets = TargetSet::only(Target::Query);
                field.overrides = Default::default();
                field.serde_always.clear();
                field.serde_typed.clear();
                field.serde_as = None;
                field.is_id = false;
                field.secret = false;
                field.sortable = false;
                field.modes = QueryModes {
                    eq: false,
                    range: false,
                    contains: false,
                };
                field.query = kind;
                field.source = Some(source.ident.clone());
                field.clearable = false;
                field.default = None;
                if !keep_rules {
                    field.rules.retain(|r| {
                        matches!(
                            r.kind,
                            rules::Kind::Length { .. }
                                | rules::Kind::Ascii
                                | rules::Kind::Alphanumeric
                                | rules::Kind::Custom(_)
                        )
                    });
                }
                fields.push(field);
            }
        }
        fields
    }
}

impl Field {
    fn parse(f: &syn::Field, entity: &Ident, patch: bool) -> Result<(Self, Option<Span>)> {
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
        let at = format!("`{entity}::{ident}`");
        if let (Some(span), Some(_)) = (attrs.sortable, attrs.skip) {
            return Err(Error::new(
                span,
                format!(
                    "{at}: `sortable` has no effect on a skipped field; remove `sortable` or `skip`"
                ),
            ));
        }
        let modes = query_modes(&attrs.overrides[Target::Query.index()], &at, f)?;
        if !attrs.normalize.is_empty() {
            if ![
                Target::Create,
                Target::Update,
                Target::Delete,
                Target::Query,
            ]
            .into_iter()
            .any(|target| targets.contains(target))
            {
                return Err(Error::new(
                    ident.span(),
                    format!("{at}: `normalize(..)` needs the field in a request DTO"),
                ));
            }
            for target in [
                Target::Create,
                Target::Update,
                Target::Delete,
                Target::Query,
            ] {
                if !targets.contains(target) {
                    continue;
                }
                let ty = attrs.overrides[target.index()]
                    .ty
                    .as_ref()
                    .map_or(&f.ty, |(_, ty)| ty);
                if !normalizable_type(ty) {
                    return Err(Error::new(
                        ty.span(),
                        format!(
                            "{at}: `normalize(..)` supports only String, Option<String> or Maybe<String>, not `{}` in the {} request",
                            show(ty),
                            target.keyword()
                        ),
                    ));
                }
            }
        }
        if attrs.serde_as.is_some()
            && let Some((span, _)) = Target::ALL
                .iter()
                .find_map(|t| attrs.overrides[t.index()].ty.as_ref())
        {
            return Err(Error::new(
                *span,
                format!(
                    "{at}: `ty = ..` cannot be combined with `serde_as`, which is written for the field's declared type; remove one of them"
                ),
            ));
        }
        let rules = match attrs.validate {
            None => Vec::new(),
            Some((span, rules)) => {
                check_rules(&rules, f, &attrs.overrides, targets, span)?;
                rules
            }
        };
        for t in Target::ALL {
            if let Some((span, rules)) = &attrs.overrides[t.index()].validate {
                if !targets.contains(t) || t == Target::Response {
                    return Err(Error::new(
                        *span,
                        format!(
                            "`{}(validate(..))` requires this field in a request DTO",
                            t.keyword()
                        ),
                    ));
                }
                check_rules(rules, f, &attrs.overrides, TargetSet::only(t), *span)?;
            }
        }

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
                if let Some((ty_span, _)) = &attrs.overrides[Target::Update.index()].ty {
                    return Err(Error::new(
                        *ty_span,
                        format!(
                            "{at}: `clearable` cannot be combined with `update(ty = ..)`; remove one of them"
                        ),
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
                    && attrs.overrides[Target::Update.index()].ty.is_none()
            }
        };

        let field = Field {
            ident: ident.clone(),
            ty: f.ty.clone(),
            docs: f
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("doc"))
                .cloned()
                .collect(),
            is_id: attrs.id.is_some(),
            targets,
            overrides: attrs.overrides,
            serde_always: serde.always,
            serde_typed: serde.typed,
            serde_as: attrs.serde_as,
            secret: attrs.secret.is_some(),
            sortable: attrs.sortable.is_some(),
            modes,
            query: QueryKind::Eq,
            source: None,
            rules,
            map_from: attrs.map_from.map(|s| s.value()),
            map_with: attrs.map_with,
            normalize: attrs.normalize.iter().map(ToString::to_string).collect(),
            default: attrs.default.map(|(_, path)| path),
            clearable,
        };
        Ok((field, attrs.id))
    }
}

/// The filters asked for by `query(..)` (equality when none is named), with
/// their type requirements checked against the field's query type.
fn query_modes(ov: &TargetOverride, at: &str, f: &syn::Field) -> Result<QueryModes> {
    let named = ov.eq.is_some() || ov.range.is_some() || ov.contains.is_some();
    let modes = QueryModes {
        eq: ov.eq.is_some() || !named,
        range: ov.range.is_some(),
        contains: ov.contains.is_some(),
    };
    let ty = ov.ty.as_ref().map_or(&f.ty, |(_, ty)| ty);
    let ty = option_inner(ty).unwrap_or(ty);
    let class = classify(ty);
    if let Some(span) = ov.range
        && matches!(class, Class::Collection | Class::Bool | Class::Char)
    {
        return Err(Error::new(
            span,
            format!(
                "{at}: `query(range)` needs an ordered type such as a number, string or date, not `{}`; use `query(eq)` or remove `range`",
                show(ty)
            ),
        ));
    }
    if let Some(span) = ov.contains
        && !matches!(class, Class::Text | Class::Unknown)
    {
        return Err(Error::new(
            span,
            format!(
                "{at}: `query(contains)` needs text (String or &str), not `{}`; use `query(range)` or `query(eq)` instead",
                show(ty)
            ),
        ));
    }
    Ok(modes)
}

/// Type-check the `validate(..)` rules against the field's type in every
/// request type that contains it. Targets without a `ty` override share the
/// entity's field type and are checked once.
fn check_rules(
    rules: &[Rule],
    f: &syn::Field,
    overrides: &[TargetOverride; 5],
    targets: TargetSet,
    span: Span,
) -> Result<()> {
    const REQUESTS: [Target; 4] = [
        Target::Create,
        Target::Update,
        Target::Delete,
        Target::Query,
    ];
    let in_create = targets.contains(Target::Create);
    if !REQUESTS.iter().any(|t| targets.contains(*t)) {
        let ctx = Ctx {
            in_create,
            in_any_request: false,
            is_create_target: false,
        };
        return rules::check(rules, &f.ty, &ctx, span);
    }

    let mut errors = None;
    // Targets using the entity's own type, then one check per override.
    let base: Vec<Target> = REQUESTS
        .into_iter()
        .filter(|t| targets.contains(*t) && overrides[t.index()].ty.is_none())
        .collect();
    if !base.is_empty() {
        let ctx = Ctx {
            in_create,
            in_any_request: true,
            is_create_target: base.contains(&Target::Create),
        };
        if let Err(e) = rules::check(rules, &f.ty, &ctx, span) {
            push_error(&mut errors, e);
        }
    }
    for t in REQUESTS {
        if let (true, Some((_, ty))) = (targets.contains(t), &overrides[t.index()].ty) {
            let ctx = Ctx {
                in_create,
                in_any_request: true,
                is_create_target: t == Target::Create,
            };
            if let Err(e) = rules::check(rules, ty, &ctx, span) {
                push_error(&mut errors, e);
            }
        }
    }
    errors.map_or(Ok(()), Err)
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
        if let Some(sortable) = attrs.sortable {
            return Err(Error::new(
                sortable,
                format!(
                    "{at}: a `secret` field cannot be sortable because sorting would expose it in the query API; remove `sortable`"
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
        if let Some((t, span)) = attrs.targets.iter().find(|(t, _)| *t != Target::Query) {
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
