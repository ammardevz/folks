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
