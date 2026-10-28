# Changelog

## 0.2.0

### Breaking changes

- Generated request types now implement `garde::Validate<Context = ()>`. The
  `Validatable` trait has been removed. Import `folks::garde::Validate` (or the
  folks prelude) and handle `garde::Report` from `validate()`; convert it with
  `folks::Error::from(report)` when the library error type is needed.
- The minimum supported Rust version is 1.91.1.

### Added

- Added `#[entity(secret)]` to redact fields from generated `Debug` output.
- Added per-request-type field type and serialized-name overrides.
- Added opt-in range and substring query fields, typed sorting, and integrated
  pagination parameters.
- Added ORM-agnostic `#[entity(from(...))]` conversions into response types.
- Added the `Dto` derive alias, generated type documentation, and optional
  schema derives for `FieldError` and `Page<T>`.

See the README for migration examples and the full attribute reference.
