//! Runtime crate for `folks`.
//!
//! `#[derive(Entity)]` is re-exported here, next to the [`Entity`] trait it
//! implements (derive macros and traits live in different namespaces, like
//! `serde::Serialize`).
//!
//! ```
//! use folks::prelude::*;
//!
//! #[derive(Entity)]
//! pub struct User {
//!     #[entity(id)]
//!     pub id: u32,
//!     #[entity(validate(length(min = 1)))]
//!     pub name: String,
//! }
//!
//! // Generated: UserResponse, UserCreateRequest, UserUpdateRequest,
//! // UserDeleteRequest, UserQueryRequest.
//! let create = UserCreateRequest { name: String::new() };
//! assert!(create.validate().is_err());
//!
//! let update = UserUpdateRequest { name: None };
//! assert!(update.is_empty()); // nothing to change
//! assert!(UserDeleteRequest { id: 1 }.validate().is_ok());
//!
//! // Fluent setters, patching and conversion are generated too.
//! let mut user = User { id: 1, name: "old".into() };
//! UserUpdateRequest::default().with_name("new").apply_to(&mut user);
//! let response: UserResponse = user.into();
//! assert_eq!(response.name, "new");
//! ```
//!
//! Cargo features: `schemars`, `utoipa` (opt-in per entity), and the extra
//! validation rules `email`, `url`, `pattern`.
//!
//! The attribute reference is in the README; the same text is on the derive
//! macro's crate page (`folks-macros`).

// Lets generated code say `::folks::...` even inside this crate's own tests.
extern crate self as folks;

pub mod core;

pub use self::core::entity::Entity;
pub use self::core::error::{Error, FieldError};
pub use self::core::filter::Filter;
pub use self::core::maybe::Maybe;
pub use self::core::page::{Page, PageRequest};
pub use self::core::patch::{Diff, Patch};
pub use self::core::sort::SortOrder;
pub use folks_macros::Entity;

// Re-exported so generated code (and `#[entity(serde_as = "...")]` types) need
// no direct `serde` / `serde_with` dependency in the consumer crate.
#[cfg(feature = "schemars")]
pub use schemars;
pub use serde;
pub use serde_with;
// Validation rules run through garde's rule functions (`folks::garde`), so
// consumers need no direct garde dependency or matching version.
pub use garde;
#[cfg(feature = "pattern")]
pub use regex;
#[cfg(feature = "utoipa")]
pub use utoipa;

/// `use folks::prelude::*;` brings in everything day-to-day code needs: the
/// derive and trait (both named `Entity`), `garde::Validate` (for
/// `.validate()`), `Patch`, `Page`.
pub mod prelude {
    pub use crate::garde::Validate;
    pub use crate::{Diff, Entity, Filter, Maybe, Page, PageRequest, Patch, SortOrder};
}
