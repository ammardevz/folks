//! Conversions between an entity field and a generated field whose type was
//! overridden with `#[entity(create(ty = ..))]` and friends.

/// Used by generated code to convert between the entity's field type and an
/// overridden field type. It is implemented for every `A: Into<T>`; it exists
/// so that a missing conversion produces a message that names the cause.
#[diagnostic::on_unimplemented(
    message = "cannot convert `{Self}` to `{T}` for a field with a `ty = ..` override",
    label = "no conversion from `{Self}` to `{T}`",
    note = "a `ty = ..` override needs `{Self}: Into<{T}>`: implement `From<{Self}> for {T}`, or change the override"
)]
pub trait ConvertField<T> {
    /// Converts `self` into the field type.
    fn convert_field(self) -> T;
}

// Hides the `Into` impl from error messages, so a missing conversion is
// reported with the message above rather than as an unsatisfied `Into` bound.
#[diagnostic::do_not_recommend]
impl<A: Into<T>, T> ConvertField<T> for A {
    fn convert_field(self) -> T {
        self.into()
    }
}
