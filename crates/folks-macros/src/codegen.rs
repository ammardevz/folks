//! Token generation: one struct + `garde::Validate` impl per target, then the
//! `Entity` impl tying them together.

use std::collections::HashSet;

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::ext::IdentExt;
use syn::{Error, Generics, Ident, LitStr, Result, parse_quote};

use crate::generics::{param_names, subset};
use crate::model::{Entity, Field, Target};
use crate::names;
use crate::rules::{self, Kind};
use crate::types::idents_in;

pub fn expand(e: &Entity) -> Result<TokenStream> {
    let krate = resolve_crate()?;
    let generated: Vec<Generated> = Target::ALL
        .iter()
        .map(|&t| generate(e, t, &krate))
        .collect();
    let defs = generated.iter().map(|g| &g.tokens);
    let entity_impl = entity_impl(e, &generated, &krate);
    // `Target::ALL` starts with `Response`.
    let from_impl = e.cfg.helpers.from.then(|| from_impl(e, &generated[0]));
    Ok(quote! { #(#defs)* #entity_impl #from_impl })
}

/// Path to the runtime crate, honouring `package = "folks"` renames.
fn resolve_crate() -> Result<TokenStream> {
    match crate_name("folks") {
        // Integration tests / examples of `folks` itself also take this path;
        // `extern crate self as folks;` makes `::folks` valid in-crate too.
        Ok(FoundCrate::Itself) => Ok(quote!(::folks)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote!(::#ident))
        }
        Err(err) => Err(Error::new(
            Span::call_site(),
            format!(
                "`#[derive(Entity)]` could not locate the `folks` crate; add it as a dependency ({err})"
            ),
        )),
    }
}

struct Generated {
    name: Ident,
    generics: Generics,
    tokens: TokenStream,
}

impl Generated {
    /// `Name<'a, T>` with only the generics this type kept.
    fn ty(&self) -> TokenStream {
        let name = &self.name;
        let (_, ty_g, _) = self.generics.split_for_impl();
        quote!(#name #ty_g)
    }
}

fn generate(e: &Entity, target: Target, krate: &TokenStream) -> Generated {
    let fields = e.fields_for(target);
    let name = e.type_name(target);

    let mut used = HashSet::new();
    for f in &fields {
        used.extend(idents_in(f.ty));
        if let Some(x) = &f.serde_as {
            used.extend(idents_in(x));
        }
    }
    let generics = subset(e.generics, &used);
    let (impl_g, _, where_c) = generics.split_for_impl();

    let vis = e.vis(target);
    let serde_path = LitStr::new(&quote!(#krate::serde).to_string(), Span::call_site());
    let serde_with_path = LitStr::new(&quote!(#krate::serde_with).to_string(), Span::call_site());
    let uses_serde_as = fields.iter().any(|f| f.serde_as.is_some());
    let serde_as_attr =
        uses_serde_as.then(|| quote!(#[#krate::serde_with::serde_as(crate = #serde_with_path)]));
    let container_serde = e.serde_attrs(target);
    let doc = format!(
        "{} type generated for `{}` by `#[derive(Entity)]`.",
        target.suffix(),
        e.ident.unraw()
    );
    let field_defs = fields
        .iter()
        .map(|f| field_def(f, target, krate, e.cfg.utoipa));

    let mut derives = vec![
        quote!(::core::fmt::Debug),
        quote!(::core::clone::Clone),
        quote!(#krate::serde::Serialize),
        quote!(#krate::serde::Deserialize),
    ];
    if e.cfg.schemars {
        derives.push(quote!(#krate::schemars::JsonSchema));
    }
    if e.cfg.utoipa {
        derives.push(quote!(::utoipa::ToSchema));
    }
    derives.extend(e.derives(target).into_iter().map(ToTokens::to_token_stream));
    let schemars_attr = e.cfg.schemars.then(|| {
        let path = LitStr::new(&quote!(#krate::schemars).to_string(), Span::call_site());
        quote!(#[schemars(crate = #path)])
    });
    let validatable = validatable_impl(e, &name, &generics, &fields, target, krate);
    let extras = match target {
        Target::Create | Target::Update | Target::Query => {
            builder_impls(e, target, &name, &generics, &fields, krate)
        }
        _ => quote!(),
    };
    let diff = (target == Target::Update && e.cfg.diff)
        .then(|| diff_impl(e, &name, &generics, &fields, krate));
    let matches = (target == Target::Query && e.cfg.matches)
        .then(|| matches_impl(e, &name, &generics, &fields, krate));
    let apply = (target == Target::Update && e.cfg.helpers.patch)
        .then(|| patch_impl(e, &name, &generics, &fields, krate));

    let tokens = quote! {
        #[doc = #doc]
        #serde_as_attr
        #[derive(#(#derives),*)]
        #schemars_attr
        #[serde(crate = #serde_path #(, #container_serde)*)]
        // Private entities get private types that may never be used.
        #[allow(dead_code)]
        #vis struct #name #impl_g #where_c {
            #(#field_defs)*
        }

        #validatable
        #extras
        #apply
        #diff
        #matches
    };
    Generated {
        name,
        generics,
        tokens,
    }
}

fn field_def(f: &Field, target: Target, krate: &TokenStream, utoipa: bool) -> TokenStream {
    let ident = f.ident;
    // Generated fields are always documented, so `#![warn(missing_docs)]`
    // does not fire on them.
    let fallback_doc = f.docs.is_empty().then(|| {
        let doc = format!("The `{}` field.", ident.unraw());
        quote!(#[doc = #doc])
    });
    let docs = &f.docs;
    let ty = f.ty;
    let optional = f.optional_in(target);
    let maybe = f.is_maybe(target);
    let inner = f.inner_ty();
    let field_ty = if maybe {
        quote!(#krate::core::maybe::Maybe<#inner>)
    } else if f.wrapped_in(target) {
        quote!(::core::option::Option<#ty>)
    } else {
        quote!(#ty)
    };

    let mut serde: Vec<TokenStream> = f
        .serde_always
        .iter()
        .map(ToTokens::to_token_stream)
        .collect();
    if target.wraps_in_option() {
        serde.push(quote!(default));
        if maybe {
            let path = LitStr::new(
                &quote!(#krate::core::maybe::Maybe::is_unset).to_string(),
                Span::call_site(),
            );
            serde.push(quote!(skip_serializing_if = #path));
        } else {
            serde.push(quote!(
                skip_serializing_if = "::core::option::Option::is_none"
            ));
        }
    } else {
        // `#[entity(default..)]` replaces any copied source `default`.
        let own_default = target == Target::Create && f.default.is_some();
        serde.extend(
            f.serde_typed
                .iter()
                .filter(|m| !(own_default && m.path().is_ident("default")))
                .map(ToTokens::to_token_stream),
        );
        if target == Target::Create {
            match &f.default {
                Some(None) => serde.push(quote!(default)),
                Some(Some(path)) => serde.push(quote!(default = #path)),
                None => {}
            }
        }
    }
    let serde_attr = (!serde.is_empty()).then(|| quote!(#[serde(#(#serde),*)]));

    // `serde_as` is declared for the inner type; wrap it in `Option` exactly
    // where the generated field is an `Option`.
    let serde_as_attr = f.serde_as.as_ref().map(|x| {
        let as_ty = if optional {
            quote!(::core::option::Option<#x>)
        } else {
            quote!(#x)
        };
        let lit = LitStr::new(&as_ty.to_string(), Span::call_site());
        quote!(#[serde_as(r#as = #lit)])
    });

    // utoipa does not know `Maybe<T>`; it is documented as a nullable `T`.
    let utoipa_attr = (utoipa && maybe).then(|| quote!(#[schema(value_type = Option<#inner>)]));

    quote! {
        #fallback_doc
        #(#docs)*
        #serde_attr
        #serde_as_attr
        #utoipa_attr
        pub #ident: #field_ty,
    }
}

/// The name `f` has on the wire in `target`, which is what clients see in
/// validation errors: serde `rename` first, then `rename_all`.
fn wire_name(e: &Entity, f: &Field, target: Target) -> String {
    names::wire_name(
        &f.ident.unraw().to_string(),
        names::field_rename(&f.serde_always),
        names::rename_all(&e.serde_attrs(target)),
    )
}

/// `garde::Validate` for the four request types, plus an inherent
/// `is_empty`. Responses are not validated: they describe data that already
/// exists.
fn validatable_impl(
    e: &Entity,
    name: &Ident,
    generics: &Generics,
    fields: &[&Field],
    target: Target,
    krate: &TokenStream,
) -> TokenStream {
    if target == Target::Response {
        return quote!();
    }
    let (impl_g, ty_g, where_c) = generics.split_for_impl();
    let garde = quote!(#krate::garde);

    // No required field and every optional field unset (vacuously true for
    // a struct without fields).
    let is_empty = if fields.iter().any(|f| !f.optional_in(target)) {
        quote!(false)
    } else {
        let checks = fields.iter().map(|f| {
            let i = f.ident;
            if f.is_maybe(target) {
                quote!(self.#i.is_unset())
            } else {
                quote!(self.#i.is_none())
            }
        });
        quote!(true #(&& #checks)*)
    };

    // An update that changes nothing is a client error.
    let empty_guard = (target == Target::Update).then(|| {
        let msg = format!("`{name}` must set at least one field");
        quote! {
            if self.is_empty() {
                #garde::Report::append(
                    __report,
                    __parent(),
                    #garde::Error::new(#msg),
                );
                return;
            }
        }
    });

    let checks: Vec<TokenStream> = fields
        .iter()
        .filter_map(|f| field_checks(e, f, target, krate))
        .collect();

    let empty_doc = match target {
        Target::Update => "Is nothing set? An empty update fails validation.",
        _ => "Is there no required field and is every optional field unset?",
    };

    quote! {
        #[allow(dead_code)]
        impl #impl_g #name #ty_g #where_c {
            #[doc = #empty_doc]
            #[must_use]
            pub fn is_empty(&self) -> bool {
                #is_empty
            }
        }

        impl #impl_g #garde::Validate for #name #ty_g #where_c {
            type Context = ();

            #[allow(unused_variables)]
            fn validate_into(
                &self,
                __ctx: &Self::Context,
                __parent: &mut dyn ::core::ops::FnMut() -> #garde::Path,
                __report: &mut #garde::Report,
            ) {
                #empty_guard
                #(#checks)*
            }
        }
    }
}

/// The checks for one field's `validate(..)` rules in one request type.
/// Optional values are checked only when present (`Some` / `Maybe::Set`);
/// `required` is enforced on the `Option` itself, in create requests only.
fn field_checks(e: &Entity, f: &Field, target: Target, krate: &TokenStream) -> Option<TokenStream> {
    if f.rules.is_empty() {
        return None;
    }
    let i = f.ident;
    let wire = LitStr::new(&wire_name(e, f, target), i.span());
    let label = quote!(__path);
    let value_ty = f.inner_ty();

    let required = (target == Target::Create && f.optional_in_source())
        .then(|| f.rules.iter().find(|r| matches!(r.kind, Kind::Required)))
        .flatten()
        .map(|r| rules::emit_required(r, &quote!(&self.#i), &label, krate));

    let value_checks = if f.is_maybe(target) {
        let body = rules::emit(&f.rules, value_ty, &quote!(__v), &label, krate);
        (!body.is_empty()).then(|| {
            quote! {
                if let #krate::core::maybe::Maybe::Set(__v) = &self.#i { #body }
            }
        })
    } else if f.optional_in(target) {
        let body = rules::emit(&f.rules, value_ty, &quote!(__v), &label, krate);
        (!body.is_empty()).then(|| {
            quote! {
                if let ::core::option::Option::Some(__v) = &self.#i { #body }
            }
        })
    } else {
        let body = rules::emit(&f.rules, value_ty, &quote!(&self.#i), &label, krate);
        (!body.is_empty()).then_some(body)
    };

    match (required, value_checks) {
        (None, None) => None,
        (r, v) => Some(quote! {
            {
                let __path = #krate::garde::Path::join(&__parent(), #wire);
                #r
                #v
            }
        }),
    }
}

/// Ergonomics for the create, update and query types:
///
/// * `Default` (update/query: everything unset, no bound on generics);
/// * create: `new(required..)`;
/// * fluent setters: `with_x(v)` (any `Into<T>`), and for optional fields
///   `with_x_opt(Option<_>)` (sets when `Some`, leaves untouched on `None`);
///   clearable update fields also get `clear_x()` and
///   `with_x_nullable(Option<_>)` (`None` clears).
fn builder_impls(
    e: &Entity,
    target: Target,
    name: &Ident,
    generics: &Generics,
    fields: &[&Field],
    krate: &TokenStream,
) -> TokenStream {
    let (impl_g, ty_g, where_c) = generics.split_for_impl();

    let default_impl = (target != Target::Create && e.cfg.helpers.default).then(|| {
        let defaults = fields.iter().map(|f| {
            let i = f.ident;
            quote!(#i: ::core::default::Default::default())
        });
        quote! {
            impl #impl_g ::core::default::Default for #name #ty_g #where_c {
                fn default() -> Self {
                    Self { #(#defaults),* }
                }
            }
        }
    });

    let new_fn = (target == Target::Create && e.cfg.helpers.new).then(|| {
        let required: Vec<_> = fields.iter().filter(|f| !f.optional_in(target)).collect();
        let params = required.iter().map(|f| {
            let i = f.ident;
            let ty = f.ty;
            quote!(#i: impl ::core::convert::Into<#ty>)
        });
        let inits = fields.iter().map(|f| {
            let i = f.ident;
            if f.optional_in(target) {
                quote!(#i: ::core::option::Option::None)
            } else {
                quote!(#i: ::core::convert::Into::into(#i))
            }
        });
        quote! {
            /// Creates a request from its required fields; optional fields
            /// start unset (use the `with_*` setters).
            #[must_use]
            #[allow(clippy::too_many_arguments, clippy::new_without_default)]
            pub fn new(#(#params),*) -> Self {
                Self { #(#inits),* }
            }
        }
    });

    let setters = e.cfg.helpers.setters.then(|| {
        fields
            .iter()
            .map(|f| setters_for(f, target, krate))
            .collect::<Vec<_>>()
    });
    let inherent = (new_fn.is_some() || setters.is_some()).then(|| {
        let setters = setters.iter().flatten();
        quote! {
            #[allow(dead_code)]
            impl #impl_g #name #ty_g #where_c {
                #new_fn
                #(#setters)*
            }
        }
    });

    quote! { #default_impl #inherent }
}

fn setters_for(f: &Field, target: Target, krate: &TokenStream) -> TokenStream {
    let i = f.ident;
    let ty = f.inner_ty();
    let maybe = f.is_maybe(target);
    let optional = f.optional_in(target);
    let name = i.unraw();
    let with = Ident::new(&format!("with_{name}"), i.span());
    let into = quote!(::core::convert::Into::into(value));

    let wrap = |expr: TokenStream| {
        if maybe {
            quote!(#krate::core::maybe::Maybe::Set(#expr))
        } else if optional {
            quote!(::core::option::Option::Some(#expr))
        } else {
            expr
        }
    };
    let assigned = wrap(into.clone());
    let doc = format!("Sets `{name}`.");

    let opt = optional.then(|| {
        let with_opt = Ident::new(&format!("with_{name}_opt"), i.span());
        let doc = format!("Sets `{name}` when `Some`; `None` leaves it as it is.");
        let assigned = wrap(into.clone());
        quote! {
            #[doc = #doc]
            #[must_use]
            pub fn #with_opt(mut self, value: ::core::option::Option<impl ::core::convert::Into<#ty>>) -> Self {
                if let ::core::option::Option::Some(value) = value {
                    self.#i = #assigned;
                }
                self
            }
        }
    });

    let nullable = maybe.then(|| {
        let clear = Ident::new(&format!("clear_{name}"), i.span());
        let nullable = Ident::new(&format!("with_{name}_nullable"), i.span());
        let clear_doc = format!("Sets `{name}` to null.");
        let nullable_doc = format!("Sets `{name}` when `Some`, and to null when `None`.");
        quote! {
            #[doc = #clear_doc]
            #[must_use]
            pub fn #clear(mut self) -> Self {
                self.#i = #krate::core::maybe::Maybe::Null;
                self
            }

            #[doc = #nullable_doc]
            #[must_use]
            pub fn #nullable(mut self, value: ::core::option::Option<impl ::core::convert::Into<#ty>>) -> Self {
                self.#i = match value {
                    ::core::option::Option::Some(value) => #krate::core::maybe::Maybe::Set(#into),
                    ::core::option::Option::None => #krate::core::maybe::Maybe::Null,
                };
                self
            }
        }
    });

    quote! {
        #[doc = #doc]
        #[must_use]
        pub fn #with(mut self, value: impl ::core::convert::Into<#ty>) -> Self {
            self.#i = #assigned;
            self
        }
        #opt
        #nullable
    }
}

/// `impl Patch<Entity> for UpdateRequest`: copies every supplied field over.
/// The impl carries all of the entity's generics (the trait reference uses
/// them), while the update type itself only names the ones it needs.
fn patch_impl(
    e: &Entity,
    name: &Ident,
    generics: &Generics,
    fields: &[&Field],
    krate: &TokenStream,
) -> TokenStream {
    let (_, sub_ty_g, _) = generics.split_for_impl();
    let (impl_g, entity_ty_g, entity_where) = e.generics.split_for_impl();
    let entity = e.ident;

    let assigns = fields.iter().map(|f| {
        let i = f.ident;
        if f.is_maybe(Target::Update) {
            return quote!(#krate::core::maybe::Maybe::apply_to(self.#i, &mut __target.#i););
        }
        let value = if f.optional_in_source() {
            quote!(::core::option::Option::Some(__value))
        } else {
            quote!(__value)
        };
        quote! {
            if let ::core::option::Option::Some(__value) = self.#i {
                __target.#i = #value;
            }
        }
    });
    let body = if fields.is_empty() {
        quote!(let _ = (self, __target);)
    } else {
        quote!(#(#assigns)*)
    };

    quote! {
        impl #impl_g #krate::core::patch::Patch<#entity #entity_ty_g> for #name #sub_ty_g #entity_where {
            fn apply_to(self, __target: &mut #entity #entity_ty_g) {
                #body
            }
        }
    }
}

/// Extra `FieldTy: Bound` predicates for field types that mention the
/// entity's generics (concrete types are checked where they are used).
fn field_bounds(e: &Entity, fields: &[&Field], bounds: TokenStream) -> Vec<TokenStream> {
    let names = param_names(e.generics);
    fields
        .iter()
        .filter(|f| !names.is_disjoint(&idents_in(f.ty)))
        .map(|f| {
            let ty = f.ty;
            quote!(#ty: #bounds)
        })
        .collect()
}

/// `impl Diff<Entity> for UpdateRequest` (opt-in `diff`).
fn diff_impl(
    e: &Entity,
    name: &Ident,
    generics: &Generics,
    fields: &[&Field],
    krate: &TokenStream,
) -> TokenStream {
    let (_, sub_ty_g, _) = generics.split_for_impl();
    let mut entity_generics = e.generics.clone();
    for pred in field_bounds(
        e,
        fields,
        quote!(::core::clone::Clone + ::core::cmp::PartialEq),
    ) {
        entity_generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#pred));
    }
    let (impl_g, entity_ty_g, entity_where) = entity_generics.split_for_impl();
    let entity = e.ident;

    let inits = fields.iter().map(|f| {
        let i = f.ident;
        let changed = quote!(__old.#i != __new.#i);
        let value = if f.is_maybe(Target::Update) {
            quote! {
                match &__new.#i {
                    ::core::option::Option::Some(__v) => {
                        #krate::core::maybe::Maybe::Set(::core::clone::Clone::clone(__v))
                    }
                    ::core::option::Option::None => #krate::core::maybe::Maybe::Null,
                }
            }
        } else if f.optional_in_source() {
            // A flat `Option<T>` can only express "now has this value".
            quote!(::core::clone::Clone::clone(&__new.#i))
        } else {
            quote!(::core::option::Option::Some(::core::clone::Clone::clone(&__new.#i)))
        };
        quote!(#i: if #changed { #value } else { ::core::default::Default::default() })
    });

    quote! {
        impl #impl_g #krate::core::patch::Diff<#entity #entity_ty_g> for #name #sub_ty_g #entity_where {
            fn diff(__old: &#entity #entity_ty_g, __new: &#entity #entity_ty_g) -> Self {
                Self { #(#inits),* }
            }
        }
    }
}

/// `impl Filter<Entity> for QueryRequest` (opt-in `matches`).
fn matches_impl(
    e: &Entity,
    name: &Ident,
    generics: &Generics,
    fields: &[&Field],
    krate: &TokenStream,
) -> TokenStream {
    let (_, sub_ty_g, _) = generics.split_for_impl();
    let mut entity_generics = e.generics.clone();
    for pred in field_bounds(e, fields, quote!(::core::cmp::PartialEq)) {
        entity_generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#pred));
    }
    let (impl_g, entity_ty_g, entity_where) = entity_generics.split_for_impl();
    let entity = e.ident;

    let checks = fields.iter().map(|f| {
        let i = f.ident;
        let eq = if f.optional_in_source() {
            quote!(__entity.#i.as_ref() == ::core::option::Option::Some(__v))
        } else {
            quote!(__entity.#i == *__v)
        };
        quote! {
            if let ::core::option::Option::Some(__v) = &self.#i {
                if !(#eq) {
                    return false;
                }
            }
        }
    });
    let body = if fields.is_empty() {
        quote!(let _ = (self, __entity); true)
    } else {
        quote!(#(#checks)* true)
    };

    quote! {
        impl #impl_g #krate::core::filter::Filter<#entity #entity_ty_g> for #name #sub_ty_g #entity_where {
            fn matches(&self, __entity: &#entity #entity_ty_g) -> bool {
                #body
            }
        }
    }
}

/// `From<Entity> for Response`: moves the response fields out.
fn from_impl(e: &Entity, response: &Generated) -> TokenStream {
    let (impl_g, ty_g, where_c) = e.generics.split_for_impl();
    let entity = e.ident;
    let response_ty = response.ty();
    let moves = e.fields_for(Target::Response).into_iter().map(|f| {
        let i = f.ident;
        quote!(#i: __entity.#i)
    });
    quote! {
        impl #impl_g ::core::convert::From<#entity #ty_g> for #response_ty #where_c {
            fn from(__entity: #entity #ty_g) -> Self {
                Self { #(#moves),* }
            }
        }
    }
}

fn entity_impl(e: &Entity, generated: &[Generated], krate: &TokenStream) -> TokenStream {
    let name = e.ident;
    let id_ty = e.id_field().ty;
    let by = |t: Target| {
        let idx = Target::ALL.iter().position(|x| *x == t).unwrap_or(0);
        generated[idx].ty()
    };
    let (response, create, update, delete, query) = (
        by(Target::Response),
        by(Target::Create),
        by(Target::Update),
        by(Target::Delete),
        by(Target::Query),
    );

    // Only bound what the trait's `Send + Sync` requirements need, and only
    // for types that actually depend on the entity's generics.
    let names = param_names(e.generics);
    let mut generics = e.generics.clone();
    if !names.is_empty() {
        let mut candidates = vec![id_ty.to_token_stream()];
        candidates.extend([&response, &create, &update, &delete, &query].map(|t| t.clone()));
        for ty in candidates {
            if !names.is_disjoint(&idents_in(&ty)) {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote!(#ty: ::core::marker::Send + ::core::marker::Sync));
            }
        }
    }
    let (impl_g, ty_g, where_c) = generics.split_for_impl();

    quote! {
        impl #impl_g #krate::core::entity::Entity for #name #ty_g #where_c {
            type EntityId = #id_ty;
            type EntityResponse = #response;
            type EntityCreateRequest = #create;
            type EntityUpdateRequest = #update;
            type EntityDeleteRequest = #delete;
            type EntityQueryRequest = #query;
        }
    }
}
