//! Test-only crate: consumes `folks` under a renamed dependency (`my_folks`),
//! with no direct serde / serde_with / garde dependency, and with the strictest
//! lints on, so any warning in generated code fails the build.
#![deny(warnings, missing_docs, unused_results)]

use my_folks::Entity;

/// A documented public entity.
#[derive(Entity, Debug)]
#[entity(
    patch,
    diff,
    matches,
    derive(PartialEq),
    strict,
    rename_all = "camelCase"
)]
pub struct Customer {
    /// Primary key.
    #[entity(id)]
    pub id: u64,
    /// Display name.
    #[entity(validate(length(min = 1, max = 64)))]
    pub name: String,
    /// Optional bio; `null` clears it.
    #[entity(validate(length(max = 280)))]
    pub bio: Option<String>,
    /// Tags.
    #[entity(validate(each(length(min = 1, max = 16))))]
    pub tags: Vec<String>,
    /// Age.
    #[entity(validate(range(min = 0, max = 150)))]
    pub age: u8,
    /// Set by the server.
    #[entity(readonly)]
    pub created_at: String,
    /// Never exposed.
    #[entity(skip)]
    pub secret: String,
}

// Undocumented and unused: private entities must not warn either.
#[derive(Entity)]
#[allow(dead_code)]
struct Internal {
    #[entity(id)]
    id: u8,
    value: String,
    #[entity(clearable)]
    note: Option<String>,
}

#[derive(Entity)]
#[entity(vis = "pub", no(from))]
#[allow(missing_docs, dead_code)]
struct Hidden {
    #[entity(id)]
    id: u8,
    #[entity(default)]
    flag: bool,
}

/// Mentions the private types so they are not reported as dead entities.
#[doc(hidden)]
pub fn touch() -> usize {
    std::mem::size_of::<Internal>() + std::mem::size_of::<Hidden>()
}

/// An entity with a secret field, to check the generated `Debug` is lint-clean.
#[derive(Entity)]
pub struct Login {
    /// Primary key.
    #[entity(id)]
    pub id: u64,
    /// Never printed.
    #[entity(secret)]
    pub password: String,
}

/// A newtype stored in the entity but exposed as plain text.
#[derive(Debug, Clone, PartialEq)]
pub struct Handle(pub String);

impl From<String> for Handle {
    fn from(value: String) -> Self {
        Handle(value)
    }
}

impl From<Handle> for String {
    fn from(value: Handle) -> Self {
        value.0
    }
}

/// An entity whose API shape differs from its stored shape.
#[derive(Entity)]
pub struct Profile {
    /// Primary key.
    #[entity(id)]
    pub id: u64,
    /// Public handle.
    #[entity(
        response(ty = String, rename = "name"),
        create(ty = String),
        update(ty = String),
        validate(length(min = 1))
    )]
    pub handle: Handle,
}

/// A paginated query entity that exercises generated range, sort and contains fields.
#[derive(Entity)]
#[entity(matches, query(paginate))]
pub struct AuditRecord {
    /// Primary key.
    #[entity(id)]
    pub id: u64,
    /// Creation timestamp.
    #[entity(readonly, query(range), sortable)]
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    /// Searchable label.
    #[entity(query(contains), validate(length(max = 80)))]
    pub label: String,
}
