# folks

[![Rust](https://img.shields.io/badge/rust-1.91.1%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](#license)

`folks` derives request and response data transfer objects (DTOs) from a
named-field Rust entity. The derive generates the DTO types, Serde and Garde
implementations, and optional helpers for updates, filtering, diffs, sorting,
and pagination.

Use `#[derive(Entity)]` or its `#[derive(Dto)]` alias. Both derive macros are
re-exported at the `folks` crate root.

`folks` does not provide database access or HTTP routing. Application code
remains responsible for persistence and business rules. The derive keeps the
Rust types at those boundaries aligned with the entity definition.

The minimum supported Rust version is **1.91.1**. The workspace uses Rust
edition 2024.

## Contents

- [Installation](#installation)
- [Quick start](#quick-start)
- [Generated DTOs](#generated-dtos)
- [Field attributes](#field-attributes)
- [Struct and DTO configuration](#struct-and-dto-configuration)
- [Validation](#validation)
- [Normalization](#normalization)
- [Field mapping and conversions](#field-mapping-and-conversions)
- [Updates and `Maybe<T>`](#updates-and-maybet)
- [Queries, sorting, and pagination](#queries-sorting-and-pagination)
- [Patching, diffs, and in-memory filtering](#patching-diffs-and-in-memory-filtering)
- [Cargo features](#cargo-features)
- [Complete attribute reference](#complete-attribute-reference)
- [Limitations and common mistakes](#limitations-and-common-mistakes)
- [Examples and tests](#examples-and-tests)
- [License](#license)

## Installation

Add `folks` as a Git dependency:

```toml
[dependencies]
folks = { git = "https://github.com/ammardevz/folks" }
```

The default feature set is empty. To use optional integrations, enable the
features your application needs:

```toml
[dependencies]
folks = { git = "https://github.com/ammardevz/folks", features = ["email", "schemars"] }
```

The derive macro is re-exported by `folks`; applications do not need a direct
dependency on `folks-macros`. Generated paths use the `serde`, `serde_with`,
and `garde` re-exports, so those crates need not be direct dependencies unless
application code uses them independently.

## Quick start

This example defines an entity, validates a create request, applies a nullable
update, and converts the entity to a response DTO:

```rust
use folks::prelude::*;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(validate(length(min = 1, max = 80)))]
    name: String,
    #[entity(clearable)]
    bio: Option<String>,
}

fn handle_create() -> Result<UserResponse, folks::Error> {
    let request = UserCreateRequest::new("Amina").with_bio("Engineer");
    request.validate()?;

    let mut user = User {
        id: 1,
        name: request.name,
        bio: request.bio,
    };
    UserUpdateRequest::default()
        .clear_bio()
        .apply_to(&mut user);

    Ok(UserResponse::from(user))
}
```

The derive creates `UserResponse`, `UserCreateRequest`, `UserUpdateRequest`,
`UserDeleteRequest`, and `UserQueryRequest`. The prelude imports the derive
and trait named `Entity`, common runtime traits and types, and
`garde::Validate`. `folks::Error` converts a Garde report, allowing `?` as
shown above. `Error` and `FieldError` are also available from the crate root.

## Generated DTOs

For an entity named `User`, the derive generates these types by default:

| Type | Purpose | Default fields |
| --- | --- | --- |
| `UserResponse` | Return entity data | Identifier and ordinary fields |
| `UserCreateRequest` | Accept create input | Ordinary fields except the identifier |
| `UserUpdateRequest` | Accept partial updates | Updatable fields, made optional |
| `UserDeleteRequest` | Identify an entity to delete | Identifier only |
| `UserQueryRequest` | Filter a collection | Ordinary fields, made optional |

Multi-word names retain their casing: `BlogPostResponse`,
`BlogPostCreateRequest`, and so on. Struct-level `name = "Prefix"` changes the
prefix used for default names; a per-DTO `name = "Type"` sets that DTO's full
name.

The entity implements `folks::Entity` with associated types for its id,
response, and four request DTOs. Every request DTO implements
`folks::garde::Validate<Context = ()>`. Responses are not validated.

Generated DTOs derive `Debug`, `Clone`, `Serialize`, and `Deserialize` unless a
secret requires a redacting `Debug` implementation. Additional derives can be
configured. Generated fields are public.

### Default field inclusion

| Source field | Response | Create | Update | Delete | Query |
| --- | :---: | :---: | :---: | :---: | :---: |
| Ordinary field | yes | yes | yes | no | yes |
| `#[entity(id)]` | yes | no | no | yes | yes |

An entity must have exactly one id. An explicit target list replaces the
ordinary field defaults. The id may be included in `response` or `query`, but
cannot be added to `create` or `update`; it remains in delete requests.

### Generated builders and conversions

Create, update, and query DTOs have fluent setters by default. The generated
create `new(...)` takes every non-`Option` create field; declared `Option`
fields start as `None`. Update and query DTOs implement `Default` with all
fields unset.

| Generated API | DTO | Behavior |
| --- | --- | --- |
| `TypeCreateRequest::new(required, ..)` | Create | Constructs from required fields, accepting `impl Into<T>`. |
| `.with_field(value)` | Create, update, query | Sets the field from a value implementing `Into<T>`. |
| `.with_field_opt(option)` | Optional fields | Sets on `Some`; `None` leaves the current value unchanged. |
| `.clear_field()` | Clearable update fields | Sets the field to `Maybe::Null`. |
| `.with_field_nullable(option)` | Clearable update fields | `Some` sets the value; `None` clears it. |
| `.is_empty()` | Request DTOs | True when no required field is present and all optional fields are unset. An empty update fails validation. |
| `From<Entity> for EntityResponse` | Entity | Moves response fields from the entity. Enabled by default. |

Because `_opt` setters accept `Option<impl Into<T>>`, specify the inner type
when passing an untyped `None`, for example `.with_name_opt(None::<String>)`.
Use `no(new)`, `no(setters)`, `no(default)`, or `no(from)` to disable the
corresponding helper where it applies.

### Serde attributes

Generated DTOs derive `Serialize` and `Deserialize`. Source struct-level
`#[serde(rename_all = "...")]` is copied unless entity configuration provides
another `rename_all`. Field-level `rename` and `alias` are copied to DTOs
containing the field. Type-specific field keys `default`, `with`,
`serialize_with`, `deserialize_with`, and `skip_serializing_if` are copied
only to response, create, and delete DTOs, where the field keeps its declared
type. Update and query fields receive their own optional Serde behavior. Other
field Serde keys are not propagated by the macro. `serde_as` is supported
separately through `serde_with`.

## Field attributes

Field attributes are written on an entity field and can be split across
multiple `#[entity(...)]` attributes. Their effects apply only to DTOs that
contain the field.

### Inclusion and field behavior

| Attribute | Purpose and behavior | Default / restriction |
| --- | --- | --- |
| `id` | Marks the entity identifier. | Required exactly once; included in response, delete, and query. |
| `skip` | Omits the field from all DTOs. | Conflicts with target lists, `readonly`, and `immutable`; cannot be used on the id. |
| `readonly` | Includes a server-owned field in response and query. | Conflicts with `immutable`, target lists other than `query`, and the id. |
| `immutable` | Includes a field in response, create, and query, but not update. | Conflicts with `readonly`, target lists other than `query`, and the id. |
| `secret` | Accepts a secret in input, omits it from response/query, redacts it in generated DTO `Debug`. | Bare `secret` means create and update. Cannot be used on id, `readonly`, `immutable`, `skip`, or `sortable` fields. |
| `response` | Includes the field only in response. | An explicit target list replaces defaults. |
| `create` | Includes the field only in create. | Cannot include the id. |
| `update` | Includes the field only in update. | Cannot include the id. |
| `delete` | Includes the field only in delete. | The id is always included in delete. |
| `query` | Includes the field only in query. | The id is always included in query. |
| `sortable` | Adds the field to the generated sort enum. | Cannot be used on skipped or secret fields. The field does not need to be a query filter. |
| `default` | Uses the field type's `Default` value when create input omits the field. | Create fields only. |
| `default = "path"` | Calls a function for a missing create field. | Function returns the generated create field type. |
| `clearable` | Makes an `Option<T>` update field a `Maybe<T>`, allowing explicit null to clear it. | `Option<T>` update fields only; incompatible with `serde_as` and update type overrides. |
| `clearable = false` | Opts the field out of struct-level `patch`. | Only useful with `#[entity(patch)]`. |
| `serde_as = "Adapter"` | Applies a `serde_with` adapter to the field's inner type. | Incompatible with type overrides and clearable fields. |

An explicit list such as `#[entity(response, create)]` includes a field in
exactly those DTOs. A target block with options also includes that target. A
validation-only block preserves the default inclusion set when used alone; if
an explicit list is present, it adds its target to that list. For example,
`#[entity(response, create(validate(length(min = 4))))]` includes the field in
response and create. `skip` still excludes the field and conflicts with target
inclusion. See [per-DTO field overrides](#per-dto-field-overrides).

### Per-DTO field overrides

The `response`, `create`, `update`, `delete`, and `query` field blocks accept
`ty = Type` and `rename = "wireName"`. Request blocks (`create`, `update`,
`delete`, and `query`) also accept `validate(...)`; response fields are not
validated. Blocks with type or name options include their target.
Validation-only blocks preserve default inclusion when used alone and add
their target when combined with an explicit target list.

```rust
use folks::Entity;

struct Email(String);

impl From<Email> for String {
    fn from(value: Email) -> Self {
        value.0
    }
}

impl From<String> for Email {
    fn from(value: String) -> Self {
        Email(value)
    }
}

#[derive(Entity)]
struct Contact {
    #[entity(id)]
    id: u64,
    #[entity(
        response(ty = String, rename = "emailAddress"),
        create(ty = String),
        update(ty = String),
    )]
    email: Email,
}
```

`ty` changes the generated field type in that DTO. Entity-to-response
conversion uses `Into`; patching an update converts the request value back to
the entity field type. When both sides are `Option`, the inner value is
converted. A missing conversion is a compiler error. `rename` changes that
DTO's Serde field name and validation error path. The `query(...)` block also
accepts filter modes described in
[Queries, sorting, and pagination](#queries-sorting-and-pagination).

Type overrides cannot be combined with `serde_as`. `update(ty = ..)` cannot be
combined with a clearable field. An update type override cannot be used with
`diff`, and a query type override cannot be used with `matches`, because those
helpers compare request fields with entity fields. Field-level Serde
attributes for the declared type are not copied to retyped fields.

## Struct and DTO configuration

Struct options go in `#[entity(...)]` on the entity and may be split across
several attributes. Shared options apply across generated DTOs; per-DTO blocks
add to or override those settings.

```rust
use folks::Entity;

#[derive(Entity)]
#[entity(
    rename_all = "camelCase",
    strict,
    derive(PartialEq),
    response(name = "UserDto", derive(Eq)),
    create(name = "NewUser"),
    query(paginate),
    patch,
)]
struct User {
    #[entity(id)]
    id: u64,
    name: String,
}
```

### Shared struct options

| Option | Purpose | Default / behavior |
| --- | --- | --- |
| `name = "Prefix"` | Sets the prefix for default generated names. | Entity's Rust name. |
| `vis = "pub(crate)"` | Sets generated DTO and sort enum visibility. | Entity visibility; may widen but not narrow below entity visibility. |
| `derive(Path, ..)` | Adds derives to generated DTOs. | None; shared and per-DTO lists accumulate. |
| `serde(meta, ..)` | Adds container Serde metadata. | None; `crate` is reserved. |
| `rename_all = "case"` | Sets generated DTO field naming. | Uses source `#[serde(rename_all = ...)]` unless entity configuration sets it. Use one entity setting per DTO. |
| `strict` / `strict = false` | Controls `deny_unknown_fields`. | Disabled; shared `strict` applies to requests, not response. Per-DTO setting overrides it. |
| `patch` | Makes eligible `Option<T>` update fields clearable. | Disabled; `clearable = false` opts a field out. `serde_as` fields are not implicitly made clearable. |
| `diff` | Generates `Diff<Entity>` for update requests. | Disabled; included fields need `Clone + PartialEq`. |
| `matches` | Generates `Filter<Entity>` for query requests. | Disabled; comparison fields need the relevant traits. |
| `schemars` | Derives `JsonSchema` for DTOs and sort enum. | Requires Cargo feature `schemars`. |
| `utoipa` | Derives `ToSchema` for DTOs and sort enum. | Requires Cargo feature `utoipa` and a direct `utoipa` dependency in the application crate. |
| `from(Source, ..)` | Generates conversions from source types to response. | No conversions unless listed; source fields default to same Rust names. |
| `no(from, setters, patch, default, new)` | Disables selected generated helpers. | Helpers are enabled by default. |

Per-DTO blocks accept `name`, `vis`, `derive`, `serde`, `rename_all`, and
`strict`. `query(...)` also accepts `paginate`. A per-DTO `name` is the full
generated type name, not a prefix. Per-DTO names and visibility take
precedence over shared values; derives accumulate; and per-DTO `strict` can
override shared `strict`. Serde attributes are appended. An entity-level
`rename_all` replaces a source struct's Serde `rename_all`; do not specify
multiple `rename_all` values for a generated DTO.

### Defaults for omitted create fields

`#[entity(default)]` uses `Default::default()` when create input omits a field.
`#[entity(default = "path::to::function")]` calls the named function instead.
The field must be included in create. These options set Serde defaults on the
create DTO only; `#[entity(default...)]` replaces a copied field-level Serde
`default` there.

## Validation

Validation uses Garde rules. The derive parses attributes and checks supported
type/argument combinations at compile time, then generates `garde::Validate`
for each request DTO containing the field. Responses are not validated. Import
`folks::garde::Validate` or use the prelude.

```rust
use folks::Entity;

#[derive(Entity)]
struct Signup {
    #[entity(id)]
    id: u64,
    #[entity(validate(length(min = 1, max = 80)))]
    name: String,
    #[entity(validate(range(min = 0, max = 150)))]
    age: u8,
    #[entity(validate(each(length(min = 1, max = 16))))]
    tags: Vec<String>,
}
```

Shared rules apply to each request DTO containing the field unless that DTO
has its own `validate(...)` override. A target override replaces the shared
rules; it does not merge with them. Rules are checked against each request
DTO's effective field type:

```rust
use folks::Entity;

#[derive(Entity)]
struct Credentials {
    #[entity(id)]
    id: u64,
    #[entity(
        validate(length(min = 8)),
        create(validate(length(min = 12))),
        update(validate(length(min = 16))),
    )]
    password: String,
}
```

Here create enforces a minimum length of 12, update enforces 16, and other
applicable request DTOs inherit the shared minimum of 8. Without an explicit
target list, the default DTO inclusion remains in effect.

### Validation rule reference

| Rule | Arguments | Supported types / behavior |
| --- | --- | --- |
| `length(...)` | At least one of `min = N`, `max = N`, `equal = N`; optional mode `simple`, `bytes`, `chars`, or `utf16`. | Text and supported collections. `equal` cannot combine with `min` or `max`; `min` cannot exceed `max`. Default mode is Garde `simple`. |
| `range(...)` | At least one of `min = N`, `max = N`; numeric literals. | Integers and floats. Integer fields require integer bounds; unsigned fields reject negative bounds. |
| `ascii` | None. | Text. |
| `alphanumeric` | None. | Text. |
| `contains("text")` | One string literal. | Text contains the string. |
| `prefix("text")` | One string literal. | Text starts with the string. |
| `suffix("text")` | One string literal. | Text ends with the string. |
| `email` | None. | Text; requires Cargo feature `email`. |
| `url` | None. | Text; requires Cargo feature `url`. |
| `pattern("regex")` | One regular expression. | Text; requires Cargo feature `pattern`. Regex syntax is checked by the macro when enabled. |
| `required` | None. | Declared `Option<T>` create field; requires a supplied create value. |
| `custom(path)` | One function path. | `fn(&T) -> Result<(), E>` where `E: Display`; deref coercion allows `&str` and `&[T]` parameters. |
| `each(rule, ..)` | One or more nested rules. | `Vec`, `VecDeque`, `HashSet`, `BTreeSet`, slices, and arrays. Errors include element indexes; `required` is not valid inside `each`. |

A custom validator can implement application-specific checks:

```rust
use folks::Entity;

fn is_slug(value: &str) -> Result<(), &'static str> {
    if value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        Ok(())
    } else {
        Err("use lowercase letters, digits, and hyphens")
    }
}

#[derive(Entity)]
struct Label {
    #[entity(id)]
    id: u64,
    #[entity(validate(custom(is_slug)))]
    slug: String,
}
```

Rules on optional values run only when a value is present. For declared
`Option<T>`, rules are checked against `T`; update/query wrapping does not add
another `Option`. On clearable `Maybe<T>`, rules run only for `Maybe::Set`.
`required` is enforced only for create requests. Id rules apply to delete and
query where the id is included.

Validation collects all failures in a `garde::Report`, using serialized names
(including Serde renames) and collection paths such as `tags[1]`. Empty updates
produce a request-level failure; empty create and query requests are valid.
`folks::Error` converts reports into `Error::Invalid(Vec<FieldError>)` or,
when only request-level failures exist, `Error::ValidationError(String)`.

## Normalization

`#[entity(normalize(...))]` transforms request values in declaration order. It
runs during request deserialization and through generated constructors and
setters; response DTOs are not normalized. Validation checks a normalized copy.
Direct assignment to a public DTO field bypasses stored-value normalization.

```rust
use folks::Entity;

#[derive(Entity)]
struct NormalizedInput {
    #[entity(id)]
    id: u64,
    #[entity(normalize(trim, lowercase))]
    email: String,
    #[entity(normalize(trim, collapse_whitespace))]
    display_name: String,
}
```

| Category | Supported operations |
| --- | --- |
| Whitespace | `trim`, `trim_start`, `trim_end`, `collapse_whitespace` |
| Casing | `lowercase`, `uppercase`, `camel_case`, `pascal_case`, `snake_case`, `kebab_case`, `screaming_snake_case`, `screaming_kebab_case`, `title_case`, `sentence_case` |

Whitespace operations may be combined and run in the order listed. At most
one casing operation may be used on a field. Supported effective types are
`String`, `Option<String>`, and `Maybe<String>` where the generated DTO shape
permits them. `None`, `Maybe::Unset`, and `Maybe::Null` are preserved. Fields
without `normalize(...)` are unchanged.

## Field mapping and conversions

`#[entity(from(SourceType))]` generates `From<SourceType> for EntityResponse`.
By default, source fields use the same Rust name as the response field and
convert through `Into::into`. Multiple source types can be listed. The `map`
attribute only affects these source-to-response conversions; mapped fields
must be included in the response DTO.

Use `map(from = "source_field")` to select a differently named source field.
Add `map(with = "path")` for an explicit conversion function. It is a normal
user-defined function that takes the selected source value **by value** and
returns the response field type:

```rust
use folks::Entity;

mod model {
    pub struct UserModel {
        pub id: u64,
        pub email_address: String,
        pub disabled_at: Option<i64>,
    }
}

mod mapping {
    pub fn is_set<T>(value: Option<T>) -> bool {
        value.is_some()
    }
}

#[derive(Entity)]
#[entity(from(crate::model::UserModel))]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(map(from = "email_address"))]
    email: String,
    #[entity(map(from = "disabled_at", with = "crate::mapping::is_set"))]
    disabled: bool,
}

fn response(model: model::UserModel) -> UserResponse {
    model.into()
}
```

The email conversion uses `Into`; the custom function receives the source's
`disabled_at` value. `map(with = "path")` without `from` uses the same-named
source field. Missing source fields or incompatible conversion functions fail
to compile. Mapping does not infer timestamp relationships or generate state
updates.

The derive also generates `From<Entity> for EntityResponse` by moving fields
from the entity. Disable this helper with `#[entity(no(from))]` to provide a
manual conversion. `no(from)` does not disable explicit `from(SourceType)`
conversions.

## Updates and `Maybe<T>`

Update and query fields are optional. A declared `T` becomes `Option<T>`; a
declared `Option<T>` remains `Option<T>` rather than becoming
`Option<Option<T>>`. For ordinary update fields, `None` means “not supplied.”

Use `#[entity(clearable)]` on an `Option<T>` field or struct-level `patch` to
represent explicit JSON `null`. A clearable update field has type `Maybe<T>`:

| JSON input | Rust value | Meaning |
| --- | --- | --- |
| Key omitted | `Maybe::Unset` | Leave the entity field unchanged. |
| `"bio": null` | `Maybe::Null` | Clear the entity field. |
| `"bio": "text"` | `Maybe::Set("text")` | Replace the entity field. |

Generated Serde attributes default omitted fields to `Unset` and skip `Unset`
when serializing the update. `Maybe<T>` also has `as_ref`, `map`,
`into_value`, `into_nested`, and `apply_to(&mut Option<T>)` methods. When
serialized directly, `Unset` and `Null` each serialize as null; the generated
update field skips `Unset` to preserve the distinction in objects.

## Queries, sorting, and pagination

Query fields are optional equality filters by default. `query(range)` adds
inclusive lower and upper bounds; `query(contains)` adds a substring filter to
a text field. `query(eq, range)` keeps equality while adding both bounds.
Derived filter names use the Rust field name, such as `created_at_from`.

```rust
use folks::Entity;

#[derive(Entity)]
#[entity(matches, query(paginate))]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(readonly, query(range), sortable)]
    created_at: i64,
    #[entity(query(contains))]
    name: String,
}
```

This query DTO has optional `created_at_from`, `created_at_to`, and
`name_contains` fields, plus `sort`, `order`, `page`, and `per_page`.
`sortable` fields become variants of `UserSortField`; Serde naming follows the
query DTO configuration. `order` is a `SortOrder`; use
`order.unwrap_or_default()` for ascending order. These types describe query
input and do not execute a database query.

`query(paginate)` adds `page` and `per_page` as `Option<u32>` and generates
`page_request()`. `PageRequest` is 1-based, defaults to 20 items, clamps limits
to 1 through 100, and calculates offsets with saturating arithmetic. `Page<T>`
contains `items`, `total`, `page`, and `per_page`; it provides `total_pages()`
and `map`, which transforms items while preserving pagination data:

```rust
use folks::{Entity, Page};

#[derive(Entity)]
#[entity(query(paginate))]
struct Account {
    #[entity(id)]
    id: u64,
}

fn response_page(
    rows: Vec<Account>,
    total: u64,
    query: &AccountQueryRequest,
) -> Page<AccountResponse> {
    Page::new(rows, total, &query.page_request()).map(AccountResponse::from)
}
```

## Patching, diffs, and in-memory filtering

`Patch<Entity>` is generated for the update request by default. Its
`apply_to(&mut entity)` method applies supplied fields. Clearable `Maybe<T>`
fields distinguish set, clear, and leave-unchanged operations.

`#[entity(diff)]` opts into `Diff<Entity>` for the update request. Its
`diff(&old, &new)` function builds an update from changed fields and requires
`Clone + PartialEq` for included entity fields. A regular `Option<T>` transition
to `None` cannot be represented as a clear without making the field clearable.

`#[entity(matches)]` opts into `Filter<Entity>` for the query request.
`matches(&entity)` evaluates supplied equality, range, and substring filters in
memory; an empty query matches every entity. This helper is useful for small
in-memory collections and tests, not database query planning.

The struct-level `patch` option controls nullable update representation;
`no(patch)` disables the generated `Patch<Entity>` implementation. These
options are independent. `diff` and `matches` are also independently opt-in
because they require comparisons and, for diffs, cloning.

## Cargo features

No features are enabled by default.

| Feature | Enables | Additional requirement |
| --- | --- | --- |
| `schemars` | Re-exports `schemars`; enables `#[entity(schemars)]`. | None beyond the feature. |
| `utoipa` | Re-exports `utoipa`; enables `#[entity(utoipa)]`. | Application crate must directly depend on `utoipa`; its derive uses `::utoipa` paths. |
| `email` | Garde's `email` validation rule. | Enable feature `email` on `folks`. |
| `url` | Garde's `url` validation rule. | Enable feature `url` on `folks`. |
| `pattern` | `pattern("...")` validation and re-export of `regex`. | Enable feature `pattern` on `folks`. |

Schema support is also selected per entity using `#[entity(schemars)]` or
`#[entity(utoipa)]`. Cargo features are crate-wide; the derive options select
which entities receive the schema derives.

## Complete attribute reference

This section consolidates supported syntax. Explanations and interactions are
provided in the sections above.

### Field attributes

| Attribute | Default behavior | Applicable DTOs / types | Example |
| --- | --- | --- | --- |
| `id` | Required id; response, delete, query. | Exactly one named field. | `#[entity(id)]` |
| `skip` | Excluded everywhere. | Conflicts with target lists and `readonly`/`immutable`. | `#[entity(skip)]` |
| `readonly` | Response and query. | Non-id field; cannot combine with target lists other than `query`. | `#[entity(readonly)]` |
| `immutable` | Response, create, query. | Non-id field; cannot combine with target lists other than `query`. | `#[entity(immutable)]` |
| `secret` | Create and update; redacted in generated `Debug`. | Not response/query/id/readonly/immutable/skip/sortable. | `#[entity(secret)]` |
| `response`, `create`, `update`, `delete`, `query` | Ordinary defaults unless a target selection is present. | Bare flags and blocks with type/name options select their targets. Validation-only request blocks augment an explicit selection. Query is optional and id has restrictions. | `#[entity(response, create)]` |
| `target(ty = T)` | Keeps the source type. | Any target; query overrides cannot combine with `matches`, update overrides cannot combine with `diff`. | `#[entity(response(ty = String))]` |
| `target(rename = "x")` | Uses Serde/source field name. | Any target. | `#[entity(create(rename = "email"))]` |
| `query(eq)` | Equality is the default. | Query fields. | `#[entity(query(eq))]` |
| `query(range)` | No bounds. | Ordered field types; inclusive `_from` and `_to`. | `#[entity(query(range))]` |
| `query(contains)` | No substring filter. | Text fields; creates `_contains`. | `#[entity(query(contains))]` |
| `sortable` | Not sortable. | Any non-skipped, non-secret field; it need not be a query filter. | `#[entity(sortable)]` |
| `default` / `default = "path"` | No generated create default. | Create field; `path` returns the create field type. | `#[entity(default)]` |
| `clearable` / `clearable = false` | `Option<T>` uses `None` as unchanged. | `Option<T>` update; false opts out of struct `patch`. | `#[entity(clearable)]` |
| `serde_as = "Adapter"` | No adapter. | Declared inner type; incompatible with `ty` and clearable. | `#[entity(serde_as = "DisplayFromStr")]` |
| `validate(rule, ..)` | No rules. | Request DTOs containing the field. | `#[entity(validate(length(min = 1)))]` |
| `target(validate(rule, ..))` | Inherits shared rules when absent. | Create, update, delete, query; replaces shared rules for that DTO and includes its target when combined with an explicit list. | `#[entity(create(validate(required)))]` |
| `normalize(op, ..)` | No normalization. | Effective type `String`, `Option<String>`, or `Maybe<String>`. | `#[entity(normalize(trim, lowercase))]` |
| `map(from = "field")` | Same-name source field. | Response field with struct `from(SourceType)`. | `#[entity(map(from = "email_address"))]` |
| `map(with = "path")` | Converts with `Into`. | Response field with struct `from(SourceType)`; function accepts source value by value. | `#[entity(map(with = "mapping::clean"))]` |
| `map(from = "field", with = "path")` | No explicit mapping. | Combines source selection and function conversion. | `#[entity(map(from = "disabled_at", with = "mapping::is_set"))]` |

Here `target` means one of `response`, `create`, `update`, `delete`, or
`query`. A validation-only block preserves ordinary inclusion defaults when
there is no explicit field list. When a field has an explicit list, a
validation-only block adds its target to the list, so no dummy `ty` or `rename`
option is needed. `skip` remains an exclusion and cannot be combined with
target inclusion.

### Struct and per-DTO options

| Syntax | Scope | Purpose / default |
| --- | --- | --- |
| `name = "Prefix"` | Struct | Prefix for generated type names; entity name by default. |
| `vis = "Visibility"` | Struct or DTO block | Generated visibility; entity visibility by default. |
| `derive(Path, ..)` | Struct or DTO block | Additional derives; shared and target lists accumulate. |
| `serde(meta, ..)` | Struct or DTO block | Additional Serde container metadata; `crate` is reserved. |
| `rename_all = "case"` | Struct or DTO block | Serde field naming; source struct `serde(rename_all)` is used unless entity configuration sets it. Use one entity setting per DTO. |
| `strict` / `strict = false` | Struct or DTO block | Adds/removes `deny_unknown_fields`; shared setting affects request DTOs only. |
| `response(..)`, `create(..)`, `update(..)`, `delete(..)`, `query(..)` | Struct | Per-DTO `name`, `vis`, `derive`, `serde`, `rename_all`, and `strict`; `query` also accepts `paginate`. |
| `query(paginate)` | Struct or query block | Adds `page`, `per_page`, and `page_request()`. |
| `patch` | Struct | Enables clearable update semantics for eligible `Option<T>` fields. |
| `diff` | Struct | Enables update diff generation; disabled by default. |
| `matches` | Struct | Enables in-memory query matching; disabled by default. |
| `from(Source, ..)` | Struct | Generates response conversions from listed source types. |
| `schemars` / `utoipa` | Struct | Enables per-entity schema derives; needs same-named Cargo feature. |
| `no(from, setters, patch, default, new)` | Struct | Disables selected helpers; helpers are enabled by default. |

### Validation arguments

| Rule | Supported arguments |
| --- | --- |
| `length` | One or more `min = u64`, `max = u64`, `equal = u64`; optional mode `simple`, `bytes`, `chars`, `utf16`. `equal` is exclusive with bounds. |
| `range` | One or both numeric literals: `min = number`, `max = number`. |
| `ascii`, `alphanumeric`, `email`, `url`, `required` | No arguments. |
| `contains`, `prefix`, `suffix`, `pattern` | Exactly one string literal; `pattern` requires the `pattern` feature. |
| `custom` | One function path. |
| `each` | One or more nested rules. |

### Normalization operations

| Whitespace | Casing |
| --- | --- |
| `trim`, `trim_start`, `trim_end`, `collapse_whitespace` | `lowercase`, `uppercase`, `camel_case`, `pascal_case`, `snake_case`, `kebab_case`, `screaming_snake_case`, `screaming_kebab_case`, `title_case`, `sentence_case` |

Whitespace and casing operations run in declaration order; only one casing
operation may be listed.

## Limitations and common mistakes

- `Entity` supports structs with named fields, not tuple structs, enums, or
  unit structs.
- `Option` detection is syntactic: `Option<T>`, `std::option::Option<T>`, and
  `core::option::Option<T>` are recognized; aliases are not.
- The macro cannot classify validation types hidden behind aliases or generic
  parameters. Rust checks those through Garde's trait bounds.
- Required derives impose trait bounds on entity and DTO fields, commonly
  `Clone`, `Debug`, `Serialize`, and `Deserialize`.
- `From<Entity> for Response` moves entity fields and is unavailable when the
  entity implements `Drop`; use `no(from)` and a manual conversion instead.
- Generated visibility can be widened, but cannot be narrower than the entity
  visibility.
- A regular `Option<T>` diff cannot express a transition to `None`; make the
  field clearable to represent deletion.
- Query ranges, substring filters, sorting, and pagination add generated field
  names. Name collisions are compile errors.
- A target-specific validation block adds its target to an explicit field list.
  Do not combine it with `skip`; use a target list containing only the DTOs
  that should receive the field.
- `map(...)` only applies to response fields converted from an explicitly
  configured source type. Custom functions are user code; they must accept the
  source value and return the response field type.

## Examples and tests

Integration tests and focused usage examples are in `crates/folks/tests`.
Compile-fail cases and expected diagnostics are in `crates/folks/tests/ui` and
feature-specific subdirectories. `crates/folks-consumer` verifies use through a
renamed dependency and a web validation integration.

From the repository root, run:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## License

The project is licensed under the [MIT License](LICENSE).
