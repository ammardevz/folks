//! `#[derive(Entity)]` for the `folks` crate. Use it through `folks::Entity`;
//! the user-facing documentation lives on that re-export.
//!
//! For `struct User` it generates `UserResponse`, `UserCreateRequest`,
//! `UserUpdateRequest`, `UserDeleteRequest` and `UserQueryRequest`, all
//! `Debug + Clone + Serialize + Deserialize`, the request types also
//! `garde::Validate<Context = ()>`, and implements
//! `folks::core::entity::Entity` for `User`.
//!
//! # Field inclusion
//!
//! | field            | response | create | update | delete | query |
//! |------------------|:--------:|:------:|:------:|:------:|:-----:|
//! | ordinary         | yes      | yes    | yes    | no     | yes   |
//! | `#[entity(id)]`  | yes      | no     | no     | yes    | yes   |
//!
//! Shorthands: `#[entity(readonly)]` = response + query (server-owned, e.g.
//! `created_at`); `#[entity(immutable)]` = response + create + query (set
//! once, never updated). Neither combines with a list, `skip` or the id.
//!
//! `#[entity(secret)]` marks a credential (password, token, API key). A bare
//! `secret` means create + update only; listing `response` or `query` on a
//! secret field, or combining it with `readonly`, `immutable`, `skip` or the
//! id, is a compile error. Every generated type that contains a secret gets a
//! hand-written `Debug` that prints `***` for it (types without secrets keep
//! the derived `Debug`), so it cannot leak through `{:?}` logging. Secrets are
//! still (de)serialized normally.
//!
//! `#[entity(skip)]` removes a field everywhere. A list such as
//! `#[entity(response, create)]` includes the field in exactly those types.
//! The id can never join create/update, is always in delete, and
//! `response`/`query` may be listed on it. `skip` + a list is an error.
//!
//! # `Option` policy
//!
//! * response/create/delete keep the declared type.
//! * update/query fields are `Option<T>`; a field declared `Option<T>` stays
//!   `Option<T>` (never `Option<Option<T>>`). `None` means "not supplied".
//!   Only a literal `Option` path is recognised (aliases are not).
//! * update/query fields get `#[serde(default, skip_serializing_if = ..)]`.
//! * Setting a nullable field to null: `#[entity(clearable)]` on an `Option<T>`
//!   field makes the *update* field a `folks::Maybe<T>` (`Unset` / `Null` /
//!   `Set(v)`; JSON: omitted / `null` / value), not `Option<Option<T>>`. The
//!   struct-level `patch` option does this for every `Option<T>` field. Not
//!   combinable with `serde_as`; `validate(..)` rules still apply (to the
//!   value, when one is set).
//!
//! # Serde
//!
//! Generated types use `#[serde(crate = "::folks::serde")]`, so consumers need
//! no direct `serde` dependency. Struct-level `#[serde(rename_all = ..)]` is
//! copied to all types. Field `rename`/`alias` go to every type including the
//! field; `default`, `with`, `serialize_with`, `deserialize_with` and
//! `skip_serializing_if` go only to response/create/delete, where the type is
//! unchanged. Other serde keys are not propagated.
//!
//! For `serde_with`, use `#[entity(serde_as = "DisplayFromStr")]`, giving the
//! adapter for the *inner* type; it is wrapped in `Option<..>` wherever the
//! generated field is optional, and `#[serde_as]` is added to the struct
//! automatically.
//!
//! # Validation
//!
//! `#[entity(validate(rule, rule, ..))]` runs rules in the request types that
//! contain the field. The request types implement `garde::Validate`
//! (re-exported as `folks::garde`), so `.validate()` collects **every**
//! failure in a `garde::Report`; `folks::Error` converts from it
//! (`Error::Invalid(Vec<FieldError>)`). Failures are reported under the field's
//! *serialized* name (serde `rename` / `rename_all`), `tags[1]` for collection
//! elements. Rules are parsed
//! and checked against the field's type at compile time, run garde's rule
//! functions through `folks::garde` (no direct garde dependency or version to
//! match), and report problems at the offending rule.
//!
//! | rule | applies to | notes |
//! |------|-----------|-------|
//! | `length(min = N, max = N, equal = N)` | text, collections | optional mode: `bytes`, `chars`, `utf16` |
//! | `range(min = N, max = N)` | integers, floats | literal bounds; no float bound on an int, no negative on unsigned |
//! | `ascii`, `alphanumeric` | text | |
//! | `contains("..")`, `prefix("..")`, `suffix("..")` | text | |
//! | `email`, `url`, `pattern("regex")` | text | cargo features of `folks`; the regex is checked at compile time |
//! | `required` | a field declared `Option<T>` | enforced in create requests only |
//! | `custom(path)` | anything | `fn(&T) -> Result<(), E: Display>`; deref coercion lets it take `&str` / `&[T]` |
//! | `each(rules..)` | `Vec`, `VecDeque`, `HashSet`, `BTreeSet`, slices, arrays | element errors are named `field[i]` |
//!
//! Rejected at compile time: a rule on a type it cannot apply to (`length` on
//! `u32`, `range` on `String`, `ascii` on a number, `each` on a non-collection
//! or a map), unknown rules and arguments (with a "did you mean"), duplicate
//! rules, `min > max`, `required` on a non-`Option` field or on a field that
//! is not in create requests, and `validate(..)` on a field that is in no
//! request type (responses are never validated). Rules on a field whose type
//! is an alias or generic cannot be classified by syntax; rustc checks those
//! through garde's trait bounds and points at the rule.
//!
//! * Rules are written against the declared type minus one `Option`. For an
//!   optional value (declared `Option<T>`, an update/query field, or a
//!   clearable `Maybe<T>`) they run only when a value is present, so `null`,
//!   unset and `None` are never "invalid".
//! * Rules on the id apply to delete and query requests.
//! * An update request that sets nothing fails with a root-level (empty path)
//!   garde error, which `folks::Error` turns into `Error::ValidationError`;
//!   empty create and query requests are valid.
//! * `custom(path)` keeps the signature `fn(&T) -> Result<(), E: Display>`;
//!   the macro adapts it to garde.
//! * The inherent `is_empty()` is true when there are no required fields and every
//!   optional field is unset.
//!
//! # Generated helpers
//!
//! * `From<User> for UserResponse` (moves the fields out; not available for
//!   entities that implement `Drop`).
//! * `UserCreateRequest::new(required..)`: required fields as `impl Into<T>`,
//!   optional ones start unset.
//! * Fluent setters on create, update and query types (`impl Into<T>` for the
//!   declared type minus one `Option`):
//!   * `with_x(v)` sets the field;
//!   * `with_x_opt(Option<_>)` (optional fields) sets when `Some`, leaves it
//!     alone on `None`, so an `Option` can be passed straight through;
//!   * clearable update fields also get `clear_x()` (null) and
//!     `with_x_nullable(Option<_>)` (`None` clears).
//! * `UserUpdateRequest` and `UserQueryRequest` implement `Default` (nothing
//!   set, no bound on generics).
//! * `Patch<User> for UserUpdateRequest`: `update.apply_to(&mut user)`.
//! * Opt-in `#[entity(diff)]`: `Diff<User> for UserUpdateRequest`,
//!   `UserUpdateRequest::diff(&old, &new)` (needs `Clone + PartialEq` fields).
//! * Opt-in `#[entity(matches)]`: `Filter<User> for UserQueryRequest`,
//!   `query.matches(&user)` tests the equality filters in memory (empty query
//!   matches everything).
//!
//! # Struct options (all optional; defaults are the behaviour above)
//!
//! Settings are layered: shared options apply to every generated type, and a
//! per-type block (`response(..)`, `create(..)`, `update(..)`, `delete(..)`,
//! `query(..)`) overrides or adds to them, so only what differs is written.
//! The attribute may be split over several `#[entity(..)]` lines.
//!
//! ```ignore
//! #[derive(Entity)]
//! #[entity(
//!     rename_all = "camelCase",                  // every type
//!     strict,                                    // deny_unknown_fields on requests
//!     derive(PartialEq),                         // every type
//!     response(name = "UserDto", derive(Eq)),    // only the response
//!     create(name = "NewUser"),
//!     patch,                                     // merge-patch updates
//!     no(from),                                  // write your own From
//! )]
//! struct User { /* .. */ }
//! ```
//!
//! Shared options: `name = ".."` (type-name prefix), `vis = ".."`,
//! `derive(..)`, `serde(..)`, `rename_all = ".."`, `strict`, `patch`,
//! `diff`, `matches`, `schemars`, `utoipa`, `no(..)`. Per-type blocks take `name`, `vis`,
//! `derive`, `serde`, `rename_all`, `strict`.
//!
//! * `strict` adds `deny_unknown_fields` to the four request types, not the
//!   response (which may come from a service that adds fields); `strict` /
//!   `strict = false` inside a block overrides that for one type.
//! * `derive` lists add up (shared, then per-type). Do not repeat `Debug`,
//!   `Clone`, `Serialize`, `Deserialize`.
//! * `vis` can widen (private entity, public DTOs); narrowing below the
//!   entity's own visibility is rejected by rustc (E0446).
//! * `serde(..)` takes any container attribute except `crate`.
//! * `no(from, setters, patch, default, new)` turns generated helpers off.
//!
//! # Patch
//!
//! `UserUpdateRequest` implements `folks::Patch<User>`: `update.apply_to(&mut
//! user)` copies every supplied field. The struct-level `patch` option turns
//! on JSON merge-patch semantics: every `Option<T>` field becomes clearable
//! (omitted = keep, `null` = clear, value = set); `clearable = false` on a
//! field opts out. Fields with `serde_as` are not made clearable implicitly.
//!
//! # Integrations (cargo features of `folks`, opt-in per entity)
//!
//! * `schemars` feature + `#[entity(schemars)]`: generated types derive
//!   `JsonSchema` through `folks::schemars` (no direct dependency needed).
//! * `utoipa` feature + `#[entity(utoipa)]`: generated types derive
//!   `utoipa::ToSchema`. utoipa's derive hardcodes `::utoipa` paths, so the
//!   crate must also depend on `utoipa` directly.
//! * Anything else: `derive(sqlx::FromRow)` etc. work through `derive(..)`,
//!   e.g. `response(derive(sqlx::FromRow))`, with that crate as a dependency.
//!
//! # Field defaults for create requests
//!
//! `#[entity(default)]` / `#[entity(default = "path::to_fn")]` fills the field
//! when a create request omits it (serde `default`). It replaces a copied
//! source `#[serde(default)]`, affects create requests only, and is an error
//! on fields that are not in them.

mod attrs;
mod codegen;
mod generics;
mod model;
mod names;
mod rules;
mod types;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Entity, attributes(entity, serde))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    model::Entity::parse(&input)
        .and_then(|entity| codegen::expand(&entity))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
