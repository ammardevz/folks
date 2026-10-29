# folks

[![Rust](https://img.shields.io/badge/rust-1.91.1%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](#license)
[![Workspace](https://img.shields.io/badge/cargo-workspace-lightgrey.svg)](https://doc.rust-lang.org/cargo/reference/workspaces.html)

A Rust library that turns one entity definition into a complete, consistent set
of request and response types, with serialization, validation, partial updates
and pagination included.

## Table of Contents

- [About](#about)
- [Features](#features)
- [Project Structure](#project-structure)
- [Installation and Setup](#installation-and-setup)
- [Quick Start](#quick-start)
- [Usage](#usage)
    - [Generated Types](#generated-types)
    - [Field Inclusion](#field-inclusion)
    - [Optional Fields and Nullable Updates](#optional-fields-and-nullable-updates)
    - [Builders and Conversions](#builders-and-conversions)
    - [Patch, Diff and Filter](#patch-diff-and-filter)
    - [Richer Query Requests](#richer-query-requests)
    - [Field Normalization](#field-normalization)
    - [Validation](#validation)
    - [Serialization](#serialization)
    - [Attribute Reference](#attribute-reference)
    - [Pagination](#pagination)
    - [Integrations](#integrations)
- [Cargo Features](#cargo-features)
- [Development](#development)
- [Design Notes and Limitations](#design-notes-and-limitations)
- [Contributing](#contributing)
- [License](#license)

## About

The minimum supported Rust version is 1.91.1.

Layered services usually need the same shape described several times: a stored
entity, a response DTO, a create request, a partial-update request, a delete
request and a query filter. Keeping those in sync by hand is repetitive and
error prone.

`folks` provides `#[derive(Entity)]` and its alias `#[derive(Dto)]`. Both use
the same `#[entity(..)]` options. `Dto` is useful when a local ORM already uses
`Entity` as a type name. From one struct with one field marked `#[entity(id)]`,
either derive generates all five types and implements
`folks::core::entity::Entity` for the struct. The generated types
derive `Debug`, `Clone`, `Serialize` and `Deserialize`, and the request types
implement `garde::Validate`, so they work with `axum-valid`, `actix-web-garde`
and every other garde integration.

Every behaviour has a default. Configuration is opt-in and layered: shared
options apply to all generated types, and per-type blocks override only what
differs.

## Features

- **Five generated types per entity**: response, create, update, delete and
  query, wired to the `Entity` trait's associated types.
- **Predictable field rules**: sensible defaults for which field appears where,
  with `skip`, `readonly`, `immutable` and explicit lists to override.
- **Correct `Option` handling**: update and query fields are never
  `Option<Option<T>>`. Nullable updates use the explicit three-state
  `Maybe<T>` (unset, null, value), which matches JSON merge-patch semantics.
- **Typed validation**: a small rule language (`length`, `range`, `email`,
  `each`, `custom` and more) checked against field types at compile time, with
  structured per-field errors.
- **Serde and serde_with support**: no direct `serde` dependency needed in
  consuming crates; field-level `serde_as` adapters are propagated correctly.
- **Ergonomic builders**: `new(..)`, `with_*`, `with_*_opt`, `clear_*` and
  `with_*_nullable` setters, `Default` for update and query types, and
  `From<Entity>` for the response.
- **Patch, Diff and Filter traits**: apply an update to an entity, compute an
  update from two entities, and match an entity against a query in memory.
- **Pagination types**: `PageRequest` and `Page<T>` with clamping and mapping.
- **Richer query requests**: opt-in inclusive ranges, substring matching,
  generated sort fields and integrated pagination parameters.
- **Optional integrations** behind Cargo features: `schemars`, `utoipa`, and
  the `email`, `url` and `pattern` validation rules.
- **Generics, lifetimes and renamed dependencies** are supported. Generated
  code is lint-clean, including for private entities.

## Project Structure

```text
.
├── Cargo.toml                  Workspace manifest
├── Cargo.lock
├── README.md
├── crates
│   ├── folks                   Runtime crate and public API
│   │   ├── Cargo.toml
│   │   ├── src
│   │   │   ├── lib.rs          Re-exports, prelude
│   │   │   └── core
│   │   │       ├── entity.rs   Entity trait
│   │   │       ├── error.rs    Error and FieldError
│   │   │       ├── filter.rs   Filter trait
│   │   │       ├── maybe.rs    Maybe<T> (unset / null / value)
│   │   │       ├── page.rs     Page and PageRequest
│   │   │       ├── patch.rs    Patch and Diff traits
│   │   │       └── mod.rs
│   │   └── tests               Integration tests and compile-fail suites
│   │       ├── entity.rs, features.rs, flex.rs, syntax.rs, comfort.rs,
│   │       │   validation.rs, integrations.rs
│   │       ├── compile.rs      trybuild driver
│   │       ├── ui/             Compile-fail cases (.rs + expected .stderr)
│   │       ├── ui_email/       Cases that apply when the email feature is off
│   │       └── ui_features/    Cases that apply when schemars is off
│   ├── folks-macros            The #[derive(Entity)] procedural macro
│   │   ├── Cargo.toml
│   │   └── src
│   │       ├── lib.rs          Entry point and macro documentation
│   │       ├── attrs.rs        Attribute parsing
│   │       ├── model.rs        Validated entity model and inclusion rules
│   │       ├── codegen.rs      Code generation
│   │       ├── rules.rs        Typed validation rule language
│   │       ├── generics.rs     Per-type generic parameter selection
│   │       └── types.rs        Syntactic type helpers
│   └── folks-consumer          Test-only crate
│       ├── Cargo.toml          Uses folks under a renamed dependency
│       ├── src/lib.rs          Built with deny(warnings, missing_docs)
│       └── tests/renamed.rs
```

| Crate            | Purpose                                                                                                                                    |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `folks`          | The crate applications depend on. Defines the traits and runtime types and re-exports the derive macro.                                    |
| `folks-macros`   | The procedural macro. Not used directly; it is re-exported by `folks`.                                                                     |
| `folks-consumer` | Not published. Verifies that the macro works through a renamed dependency and that generated code produces no warnings under strict lints. |

## Installation and Setup

### Prerequisites

- Rust 1.91.1 or newer (the workspace uses edition 2024).
- Cargo, installed with the Rust toolchain.

### Using folks in a project

Add the dependency to the `Cargo.toml` of your project:

```toml
[dependencies]
folks = { git = "https://github.com/ammardevz/folks" }
```

Enable optional integrations through Cargo features (see
[Cargo Features](#cargo-features)):

```toml
[dependencies]
folks = { git = "https://github.com/ammardevz/folks", features = ["email", "schemars"] }
```

No direct dependency on `serde` or `serde_with` is required. The generated code
refers to them through `folks`.

### Building from source

```bash
git clone https://github.com/ammardevz/folks.git
cd folks
cargo build --workspace
cargo test --workspace
```

To exercise the optional integrations as well:

```bash
cargo test --workspace --all-features
```

## Quick Start

```rust
use folks::prelude::*;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(validate(length(min = 1, max = 80)))]
    name: String,
    bio: Option<String>,
}

fn create_user() -> Result<(), folks::Error> {
    let request = UserCreateRequest::new("Amina").with_bio("Engineer");
    request.validate()?;
    Ok(())
}
```

This one definition generates `UserResponse`, `UserCreateRequest`,
`UserUpdateRequest`, `UserDeleteRequest` and `UserQueryRequest`. The sections
below show how to customize which fields appear and how each DTO behaves.

## Usage

### Generated Types

For an entity named `User`, the derive generates the following types. The names
follow the entity name and apply to multi-word names as well
(`BlogPost` produces `BlogPostResponse`, and so on).

| Type                | Purpose                  | Contents                                                |
| ------------------- | ------------------------ | ------------------------------------------------------- |
| `UserResponse`      | Data returned to clients | `id` and the ordinary fields                            |
| `UserCreateRequest` | Creating an entity       | Ordinary fields, without the id                         |
| `UserUpdateRequest` | Partial update           | Updatable fields, all optional                          |
| `UserDeleteRequest` | Deleting an entity       | The id                                                  |
| `UserQueryRequest`  | Filtering                | Optional equality, range and substring filters, plus opted-in sort/page fields |

`folks::core::entity::Entity` is implemented for `User` with:

| Associated type       | Value                                        |
| --------------------- | -------------------------------------------- |
| `EntityId`            | The type of the field marked `#[entity(id)]` |
| `EntityResponse`      | `UserResponse`                               |
| `EntityCreateRequest` | `UserCreateRequest`                          |
| `EntityUpdateRequest` | `UserUpdateRequest`                          |
| `EntityDeleteRequest` | `UserDeleteRequest`                          |
| `EntityQueryRequest`  | `UserQueryRequest`                           |

Exactly one field must carry `#[entity(id)]`. A missing or duplicated id is a
compile error. Responses are not validated; only the four request types
implement `garde::Validate` (`folks::garde::Validate`, also in the prelude).

### Field Inclusion

| Field           | response | create | update | delete | query |
| --------------- | :------: | :----: | :----: | :----: | :---: |
| Ordinary field  |   yes    |  yes   |  yes   |   no   |  yes  |
| `#[entity(id)]` |   yes    |   no   |   no   |  yes   |  yes  |

Field attributes adjust the defaults:

| Attribute                                         | Effect                                                |
| ------------------------------------------------- | ----------------------------------------------------- |
| `skip`                                            | Excluded from every generated type                    |
| `readonly`                                        | Response and query only (for example `created_at`)    |
| `immutable`                                       | Response, create and query; never updated             |
| `secret`                                          | Create and update only; redacted (`***`) in `Debug`   |
| `response`, `create`, `update`, `delete`, `query` | A list: the field appears in exactly the listed types |

The id can never be placed in create or update requests, and it is always part
of the delete request. Conflicting combinations, such as `skip` with a list, are
reported as compile errors at the relevant attribute.

#### Per-type overrides

The API shape does not have to equal the stored shape. Each list flag accepts
options:

```rust,ignore
pub struct Email(String);                 // From<String> and Into<String> exist

#[entity(
    response(ty = String, rename = "emailAddress"),
    create(ty = String),
    update(ty = String),
    validate(length(min = 3)),            // checked against `String`
)]
pub email: Email,
```

- `ty = Type` changes the field's type in that generated type only. The flag
  still means "include the field"; the bare flag keeps working.
- `rename = "x"` changes the serde name in that type only (and the name used in
  validation errors).
- `From<Entity> for Response` converts with `Into` (an `Option<A>` to
  `Option<B>` converts the inner value); `apply_to` converts update values back.
  If no conversion exists, the error points at the `ty` and says which `From`
  is missing.
- Validation rules are checked and run against the overridden type.
- Not combinable with `serde_as`, with `clearable` on `update(ty = ..)`, or
  with the struct-level `diff` (`update`) / `matches` (`query`).
- Serde keys written for the declared type (`with`, `default = ..`) are not
  copied to a retyped field. `#[entity(default = "path")]` is, so the function
  must return the create type.

#### Secrets

Generated types derive `Debug`, which would print a password into your logs.
Mark credentials with `#[entity(secret)]`:

```rust,ignore
#[entity(secret)]
pub password: String,                 // create + update only
#[entity(secret, create)]
pub api_key: Option<String>,          // create only
```

Every generated type that contains a secret gets a hand-written `Debug` that
prints `***` for it, so `{:?}` and `tracing` output never contain the value.
Types without secrets keep the derived `Debug`. A secret is still serialized
and deserialized normally. `secret` is a compile error on the id, on a field
listed in `response` or `query`, and together with `readonly`, `immutable` or
`skip`.

Note: `Debug` of your own entity struct is yours; derive or implement it
yourself with the same care.

### Optional Fields and Nullable Updates

- Response, create and delete fields keep the declared type.
- Update and query fields are `Option<T>`. A field already declared
  `Option<T>` stays `Option<T>`; it is never wrapped twice.
- By default, `None` in an update means "not supplied", so a nullable field
  cannot be set to null.

To allow setting a nullable field to null, mark it `#[entity(clearable)]`, or
enable `patch` on the struct to apply this to every `Option<T>` field. The
update field then becomes `folks::Maybe<T>`:

| JSON           | `Maybe<T>`   | Meaning                   |
| -------------- | ------------ | ------------------------- |
| key omitted    | `Unset`      | Leave the field unchanged |
| `"key": null`  | `Null`       | Clear the field           |
| `"key": value` | `Set(value)` | Replace the field         |

`#[entity(clearable = false)]` opts a field out of `patch` mode. Clearable fields
cannot be combined with `serde_as`.

### Builders and Conversions

| Generated item                | On                      | Description                                                    |
| ----------------------------- | ----------------------- | -------------------------------------------------------------- |
| `new(required..)`             | create                  | Required fields as `impl Into<T>`; optional fields start unset |
| `with_x(v)`                   | create, update, query   | Sets a field from any `Into<T>`                                |
| `with_x_opt(Option<_>)`       | optional fields         | Sets when `Some`; leaves the field untouched on `None`         |
| `clear_x()`                   | clearable update fields | Sets the field to null                                         |
| `with_x_nullable(Option<_>)`  | clearable update fields | `Some` sets; `None` clears                                     |
| `Default`                     | update, query           | Nothing set; no bounds on generic parameters                   |
| `From<User> for UserResponse` | entity                  | Moves the response fields out of the entity                    |
| `From<Model> for UserResponse` | each `from(Model)`      | Converts same-named response fields with `Into::into`          |

The entity conversion can be disabled with `#[entity(no(from))]`, for example
to provide a hand-written `From` implementation. Explicit model conversions
are independent of that helper:

```rust
#[derive(Entity)]
#[entity(from(crate::database::UserModel), from(crate::legacy::User))]
struct User {
    #[entity(id)]
    id: u64,
    name: String,
    nickname: Option<String>,
}
```

Each `from(..)` generates `From<Source> for UserResponse`, mapping fields by
their Rust names and converting each value with `Into::into`. Optional response
fields map `Option<A>` to `Option<B>` with `Option::map`. A missing source
field or a missing conversion is reported by rustc at the corresponding entity
field. Multiple source types are supported.

### Patch, Diff and Filter

| Trait          | Generated for       | Method                                                                       |
| -------------- | ------------------- | ---------------------------------------------------------------------------- |
| `Patch<User>`  | `UserUpdateRequest` | `update.apply_to(&mut user)` copies every supplied field                     |
| `Diff<User>`   | `UserUpdateRequest` | `UserUpdateRequest::diff(&old, &new)` builds the update between two entities |
| `Filter<User>` | `UserQueryRequest`  | `query.matches(&user)` tests the equality filters in memory                  |

`Patch` is always generated (disable with `no(patch)`). `Diff` and `Filter` are
opt-in with `#[entity(diff)]` and `#[entity(matches)]`, because they require
`Clone` and `PartialEq` on the field types.

### Richer Query Requests

Query fields are optional equality filters by default. Add modes to opt into
inclusive range bounds or text substring filters. Mark fields `sortable` to
generate a typed sort enum. `query(paginate)` adds page parameters directly to
the same request, so one extractor handles filtering, sorting and pagination.

```rust
#[derive(Entity)]
#[entity(matches, query(paginate))]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(readonly, query(range), sortable)]
    #[serde(rename = "createdAt")]
    created_at: i64,
    #[entity(query(eq))]
    email: String,
}
```

This accepts a request such as:

```text
GET /users?email=alice%40example.com&created_from=1700000000&sort=createdAt&order=desc&page=2
```

`query(eq, range)` exposes both `<field>` and `<field>_from` / `<field>_to`.
`query(contains)` exposes `<field>_contains` for text. `sort` is an optional
`UserSortField`; `order` is an optional `SortOrder`, and
`order.unwrap_or_default()` gives ascending order. `page_request()` converts
the optional `u32` values into a `PageRequest`, defaulting to page 1 and
`DEFAULT_PER_PAGE`; the page size is clamped by `PageRequest`.

With `matches`, equality, range and substring filters are all evaluated in
memory. Fields with `validate(..)` rules retain those rules on range bounds;
substring filters keep length, ASCII, alphanumeric and custom rules.

### Field Normalization

`normalize(..)` transforms request values in order during deserialization and
through generated constructors and setters. It supports `String`,
`Option<String>` and clearable `Maybe<String>` fields; absent attributes leave
values untouched.

```rust
#[entity(normalize(trim, lowercase))]
email: String,
#[entity(normalize(trim, collapse_whitespace))]
name: String,
#[entity(normalize(snake_case))]
permission: String,
```

Whitespace operations are `trim`, `trim_start`, `trim_end` and
`collapse_whitespace`. Casing operations are `lowercase`, `uppercase`,
`camel_case`, `pascal_case`, `snake_case`, `kebab_case`,
`screaming_snake_case`, `screaming_kebab_case`, `title_case` and
`sentence_case`. At most one casing operation may be specified. Public fields
can still be assigned directly, which bypasses normalization; use the generated
constructors/setters or deserialize request data to store normalized input.

### Validation

Validation is configured per field with `#[entity(validate(rule, rule, ..))]`.
Rules are parsed and checked against the field type at compile time, and they
run through garde's rule functions re-exported by `folks`, so consuming crates
do not need a `garde` dependency.

```rust
#[entity(validate(length(min = 1, max = 64)))]            name: String,
#[entity(validate(range(min = 0, max = 150)))]            age: u8,
#[entity(validate(required, length(max = 280)))]          bio: Option<String>,
#[entity(validate(each(length(min = 1, max = 16))))]      tags: Vec<String>,
#[entity(validate(custom(is_slug)))]                      slug: String,
```

Target blocks on a field may replace the shared rules for one request type:

```rust
#[entity(validate(length(min = 8)), create(validate(length(min = 12))), update(validate(length(min = 16))))]
password: String,
```

The shared rules apply to request DTOs without an override; an explicit
`create(validate(..))`, `update(validate(..))`, `delete(validate(..))` or
`query(validate(..))` replaces them for that DTO.

| Rule                                             | Applies to                                               | Notes                                                                                         |
| ------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `length(min, max, equal)`                        | Text, collections                                        | Optional mode: `bytes`, `chars`, `utf16`                                                      |
| `range(min, max)`                                | Integers, floats                                         | Literal bounds                                                                                |
| `ascii`, `alphanumeric`                          | Text                                                     |                                                                                               |
| `contains("..")`, `prefix("..")`, `suffix("..")` | Text                                                     |                                                                                               |
| `email`, `url`, `pattern("regex")`               | Text                                                     | Require the Cargo feature of the same name; the regular expression is checked at compile time |
| `required`                                       | Fields declared `Option<T>`                              | Enforced in create requests                                                                   |
| `custom(path)`                                   | Any type                                                 | `fn(&T) -> Result<(), E>` where `E: Display`; deref coercion allows `&str` and `&[T]`         |
| `each(rules..)`                                  | `Vec`, `VecDeque`, `HashSet`, `BTreeSet`, slices, arrays | Element failures are named `field[index]`                                                     |

Behaviour:

- Rules are written against the declared type with one `Option` layer removed.
  For optional values (declared `Option<T>`, update and query fields, and
  clearable `Maybe<T>`), rules run only when a value is present.
- Rules on the id apply to delete and query requests.
- The request types implement `garde::Validate<Context = ()>`. `validate()`
  reports every failure at once in a `garde::Report`, so
  `Garde<Json<UserCreateRequest>>` (axum-valid) and similar extractors work.
- Failures are reported under the field's **serialized** name: serde `rename`
  and `rename_all` (including a per-type `rename_all`) are applied, and
  collection elements are `field[index]`.
- `folks::Error` implements `From<garde::Report>`, so `request.validate()?`
  works in a handler returning `folks::Error`. The result is
  `Error::Invalid(Vec<FieldError>)` (each `FieldError` has a serializable
  `field` and `message`).
- Request-level problems, such as an update request that sets nothing, are
  root-level garde errors (empty path). A report with only such entries
  becomes `Error::ValidationError(String)`.
- `custom(path)` keeps the signature `fn(&T) -> Result<(), E>`; you do not
  have to use garde's `(&T, &Ctx)` signature.
- The inherent `is_empty()` is true when there are no required fields and every optional
  field is unset. An empty update fails `validate()`; empty create and query
  requests are valid.

The following are rejected at compile time, with the error pointing at the rule:
a rule on a type it cannot apply to (`length` on an integer, `range` on a
string), unknown rules and arguments (with a suggestion), duplicate rules,
`min` greater than `max`, `required` on a non-`Option` field, and `validate` on
a field that appears in no request type.

### Migrating from `Validatable`

`Validatable` is gone; the generated request types implement
`garde::Validate` instead.

| Before                                       | After                                                                                  |
| -------------------------------------------- | -------------------------------------------------------------------------------------- |
| `use folks::Validatable;`                    | `use folks::garde::Validate;` (or `folks::prelude::*`)                                 |
| `req.validate() -> Result<(), folks::Error>` | `req.validate() -> Result<(), garde::Report>`; use `?` or `folks::Error::from(report)` |
| `req.is_valid()` / `req.is_invalid()`        | `req.validate().is_ok()` / `.is_err()`                                                 |
| `req.is_empty()`                             | unchanged (now an inherent method)                                                     |
| error `field` was the Rust field name        | error `field` is the serialized name (`rename`, `rename_all`)                          |

### Serialization

Generated types derive `Serialize` and `Deserialize` using
`#[serde(crate = "::folks::serde")]`.

- A `#[serde(rename_all = ..)]` on the source struct is copied to every generated
  type.
- Field-level `rename` and `alias` are copied to every type containing the field.
- `default`, `with`, `serialize_with`, `deserialize_with` and
  `skip_serializing_if` are copied only to response, create and delete types,
  where the field keeps its declared type.
- Other serde keys are not propagated.
- For `serde_with`, use `#[entity(serde_as = "DisplayFromStr")]`, naming the
  adapter for the inner type. It is wrapped in `Option<..>` automatically where
  the generated field is optional, and `#[serde_as]` is added to the generated
  struct as needed.

### Attribute Reference

Struct-level options go in `#[entity(..)]`, which may be split across several
attributes. Shared options apply to every generated type; a per-type block
(`response(..)`, `create(..)`, `update(..)`, `delete(..)`, `query(..)`) adds to
or overrides them.

```rust
#[derive(Entity)]
#[entity(
    rename_all = "camelCase",                // every type
    strict,                                  // deny_unknown_fields on requests
    derive(PartialEq),                       // every type
    response(name = "UserDto", derive(Eq)),  // only the response
    create(name = "NewUser"),
    patch,                                   // JSON merge-patch updates
    no(from),                                // provide your own From
)]
pub struct User { /* .. */ }
```

| Option               | Description                                                                  |
| -------------------- | ---------------------------------------------------------------------------- |
| `name = ".."`        | Prefix of the generated type names                                           |
| `vis = ".."`         | Visibility of generated types (default: the entity's; may widen, not narrow) |
| `derive(..)`         | Additional derives; lists from shared options and per-type blocks add up     |
| `serde(..)`          | Additional container-level serde attributes (`crate` is reserved)            |
| `rename_all = ".."`  | Shorthand for `serde(rename_all = ..)`                                       |
| `strict`             | `deny_unknown_fields` on the four request types, not on the response         |
| `patch`              | Every `Option<T>` field becomes clearable                                    |
| `diff`, `matches`    | Opt-in `Diff` and `Filter` implementations                                   |
| `schemars`, `utoipa` | Opt-in schema derives (require the matching Cargo feature)                   |
| `from(Type)`         | Generates `From<Type> for Response`; repeat for multiple source types        |
| `no(..)`             | Disables `from`, `setters`, `patch`, `default` or `new`                      |

Per-type blocks accept `name`, `vis`, `derive`, `serde`, `rename_all` and
`strict`. `query(paginate)` is also available inside the query block.

#### Field attributes

| Attribute | Purpose |
| --- | --- |
| `id` | Marks the required identifier field. |
| `skip` | Excludes a field from generated DTOs. |
| `readonly` / `immutable` | Restricts a field to response/query, or response/create/query, respectively. |
| `secret` | Accepts a field on input, omits it from responses and redacts generated `Debug` output. |
| `sortable` | Adds a field to the generated query sort enum. |
| `response`, `create`, `update`, `delete`, `query` | Selects the DTOs that contain a field. Each target can accept `ty = Type` and `rename = "wireName"`; query also accepts `eq`, `range` and `contains`. |
| `default` / `default = "path"` | Supplies a create value when the field is omitted. The function form returns the create field type. |
| `clearable` / `clearable = false` | Enables or opts out of null-clearing for an `Option<T>` update field. |
| `serde_as = "Adapter"` | Applies a `serde_with` adapter to the field. |
| `validate(..)` | Adds shared Garde rules to applicable request DTOs. |
| `create(validate(..))`, `update(validate(..))`, `delete(validate(..))`, `query(validate(..))` | Replaces shared validation rules for that request DTO. A validation-only block keeps the field’s default inclusion. |
| `normalize(..)` | Normalizes request values during deserialization, generated setters, and validation. |
| `map(..)` | Maps a source-model field in the `from(SourceType)` response conversion. |

Most attributes can be combined when their purposes do not conflict. For
example, this field is immutable, renamed on the wire, and validated:

```rust
#[entity(immutable, validate(length(min = 2, max = 80)))]
#[serde(rename = "displayName")]
name: String,
```

#### Customizing one generated DTO

Use a target block when a request or response needs a different field type or
wire name. These options also include the field in that target DTO:

```rust
#[entity(
    response(ty = String, rename = "emailAddress"),
    create(ty = String),
    update(ty = String),
)]
email: Email,
```

`ty = Type` changes the generated field type; conversion uses `Into` for
responses and `From` for applying updates. `rename = ".."` changes that DTO’s
serde name and its validation error path. Validation-only target blocks are an
exception: they replace rules without changing normal field inclusion.

#### Mapping a source model

Declare one or more source types on the entity. Fields map by the same Rust
name by default. Use `from` to select a different source field and `with` when
the value needs an explicit conversion:

```rust
#[derive(Entity)]
#[entity(from(crate::model::UserModel))]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(map(from = "email_address"))]
    email: String,
    #[entity(map(from = "disabled_at", with = "crate::mapping::is_disabled"))]
    disabled: bool,
}
```

The conversion function takes the selected source field by value and returns
the response field type. Mapping applies only to `From<Source> for Response`.

#### Normalizing request fields

List operations in the order they should run. Whitespace operations are
`trim`, `trim_start`, `trim_end` and `collapse_whitespace`. Casing operations
are `lowercase`, `uppercase`, `camel_case`, `pascal_case`, `snake_case`,
`kebab_case`, `screaming_snake_case`, `screaming_kebab_case`, `title_case` and
`sentence_case`. Use at most one casing operation per field:

```rust
#[entity(normalize(trim, lowercase))]
email: String,
#[entity(normalize(trim, collapse_whitespace))]
name: String,
```

Supported field types are `String`, `Option<String>` and `Maybe<String>` where
the generated DTO shape permits them. A direct assignment to a public field
does not rewrite its stored value; validation checks a normalized copy.

`#[entity(default)]` and `#[entity(default = "path::to_fn")]` fill a field when
a create request omits it.

### Pagination

`PageRequest` is a 1-based, clamped page request. `offset()` and `limit()` are
always safe to hand to a data store: the page size defaults to 20 and is capped
at 100. `Page<T>` carries `items`, `total`, `page` and `per_page`, provides
`total_pages()`, and converts items with `map`:

```rust
let page = Page::new(users, total, &page_request).map(UserResponse::from);
```

### Integrations

| Integration   | Enable                                               | Notes                                                                                     |
| ------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| schemars      | Cargo feature `schemars`, then `#[entity(schemars)]` | Derives `JsonSchema` through `folks::schemars`; no extra dependency                       |
| utoipa        | Cargo feature `utoipa`, then `#[entity(utoipa)]`     | Derives `ToSchema`; utoipa hardcodes its paths, so the crate must also depend on `utoipa` |
| Other derives | `derive(..)` in a type block                         | For example `response(derive(sqlx::FromRow))`, with that crate as a dependency            |

Integrations are opt-in per entity rather than automatic, because Cargo features
are unified across a build and would otherwise affect unrelated crates.

## Cargo Features

No feature is enabled by default.

| Feature    | Description                                             |
| ---------- | ------------------------------------------------------- |
| `schemars` | Re-exports `schemars` and enables `#[entity(schemars)]` |
| `utoipa`   | Re-exports `utoipa` and enables `#[entity(utoipa)]`     |
| `email`    | Enables the `email` validation rule                     |
| `url`      | Enables the `url` validation rule                       |
| `pattern`  | Enables the `pattern("..")` rule and re-exports `regex` |

## Development

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test --workspace --all-features
```

The suite covers behaviour tests for each feature, a lint-strict consumer crate
that depends on `folks` under a renamed dependency, and compile-fail tests using
`trybuild`. Each case in `crates/folks/tests/ui` is a `.rs` file with an expected
`.stderr` file, so diagnostics are part of the tested contract.

When a diagnostic changes intentionally, review the output and regenerate the
expected files:

```bash
TRYBUILD=overwrite cargo test -p folks --test compile
```

Expected `.stderr` files reflect the compiler version used to generate them.
Regenerate them after reviewing the differences if you use a different
toolchain.

## Design Notes and Limitations

- Only structs with named fields are supported.
- `Option` detection is syntactic: only `Option`, `std::option::Option` and
  `core::option::Option` are recognised, not type aliases.
- Validation rules on aliased or generic field types cannot be classified by
  the macro; the compiler checks those through the rule's trait bounds and
  reports the error at the rule.
- Generated types and the `Entity` implementation require that field types
  support the derives in use (`Debug`, `Clone`, `Serialize`, `Deserialize`).
- `From<Entity> for Response` moves fields out of the entity, so it is not
  available for entities that implement `Drop`. Disable it with `no(from)`.
- `vis` can widen the visibility of generated types but cannot narrow it below
  the entity's own visibility.
- With `Diff`, a flat `Option<T>` field that became `None` cannot be expressed
  unless the field is clearable.
- The workspace targets Rust 1.91.1 and edition 2024. Other toolchains are not
  tested.

## Contributing

Contributions are welcome. Please open an issue to discuss significant changes
before submitting a pull request. Before submitting, ensure that formatting,
`clippy` and the full test suite pass with and without `--all-features`, and
that any intentional diagnostic change is accompanied by regenerated `.stderr`
files.

## License

Licensed under MIT License.
