//! A three-state value for partial updates (JSON merge-patch semantics).

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A field of an update request that can be left alone, cleared, or set.
///
/// This is what `Option<Option<T>>` is usually used for, with names that say
/// what each state means:
///
/// | JSON            | value          |
/// |-----------------|----------------|
/// | key omitted     | `Maybe::Unset` |
/// | `"key": null`   | `Maybe::Null`  |
/// | `"key": "text"` | `Maybe::Set("text")` |
///
/// Use it with `#[serde(default)]` (the derive does) so a missing key becomes
/// `Unset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Maybe<T> {
    /// Not supplied: leave the target as it is.
    #[default]
    Unset,
    /// Explicit null: clear the target.
    Null,
    /// A new value.
    Set(T),
}

impl<T> Maybe<T> {
    pub fn is_unset(&self) -> bool {
        matches!(self, Maybe::Unset)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Maybe::Null)
    }

    pub fn is_set(&self) -> bool {
        matches!(self, Maybe::Set(_))
    }

    pub fn as_ref(&self) -> Maybe<&T> {
        match self {
            Maybe::Unset => Maybe::Unset,
            Maybe::Null => Maybe::Null,
            Maybe::Set(v) => Maybe::Set(v),
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Maybe<U> {
        match self {
            Maybe::Unset => Maybe::Unset,
            Maybe::Null => Maybe::Null,
            Maybe::Set(v) => Maybe::Set(f(v)),
        }
    }

    /// The new value, if one was supplied (`Unset` and `Null` give `None`).
    pub fn into_value(self) -> Option<T> {
        match self {
            Maybe::Set(v) => Some(v),
            _ => None,
        }
    }

    /// `Option<Option<T>>` view: `None` = unset, `Some(None)` = null.
    pub fn into_nested(self) -> Option<Option<T>> {
        match self {
            Maybe::Unset => None,
            Maybe::Null => Some(None),
            Maybe::Set(v) => Some(Some(v)),
        }
    }

    /// Applies this to an optional target: unset keeps it, null clears it,
    /// a value replaces it.
    pub fn apply_to(self, target: &mut Option<T>) {
        match self {
            Maybe::Unset => {}
            Maybe::Null => *target = None,
            Maybe::Set(v) => *target = Some(v),
        }
    }
}

// No blanket `From<T>` / `From<Option<T>>`: they overlap and make inference
// ambiguous. Use the explicit constructors below.
impl<T> Maybe<T> {
    /// `Maybe::Set(value)`.
    pub fn set(value: impl Into<T>) -> Self {
        Maybe::Set(value.into())
    }

    /// `Some(v)` sets, `None` clears (null); never `Unset`.
    pub fn nullable(value: Option<T>) -> Self {
        value.map_or(Maybe::Null, Maybe::Set)
    }
}

impl<T> From<Option<Option<T>>> for Maybe<T> {
    fn from(value: Option<Option<T>>) -> Self {
        match value {
            None => Maybe::Unset,
            Some(None) => Maybe::Null,
            Some(Some(v)) => Maybe::Set(v),
        }
    }
}

/// `Unset` and `Null` both serialize as null; skip `Unset` fields with
/// `skip_serializing_if = "Maybe::is_unset"` (the derive does).
impl<T: Serialize> Serialize for Maybe<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Maybe::Set(v) => serializer.serialize_some(v),
            Maybe::Unset | Maybe::Null => serializer.serialize_none(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Maybe<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Option::<T>::deserialize(deserializer)?.map_or(Maybe::Null, Maybe::Set))
    }
}

#[cfg(feature = "schemars")]
impl<T: schemars::JsonSchema> schemars::JsonSchema for Maybe<T> {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        <Option<T>>::schema_name()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        <Option<T>>::schema_id()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <Option<T>>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Patch {
        #[serde(default, skip_serializing_if = "Maybe::is_unset")]
        bio: Maybe<String>,
    }

    #[test]
    fn json_round_trips_the_three_states() {
        let unset: Patch = serde_json::from_str("{}").unwrap();
        let null: Patch = serde_json::from_str(r#"{"bio":null}"#).unwrap();
        let set: Patch = serde_json::from_str(r#"{"bio":"x"}"#).unwrap();
        assert_eq!(
            (unset.bio.clone(), null.bio.clone(), set.bio.clone()),
            (Maybe::Unset, Maybe::Null, Maybe::Set("x".to_string()))
        );
        assert_eq!(serde_json::to_string(&unset).unwrap(), "{}");
        assert_eq!(serde_json::to_string(&null).unwrap(), r#"{"bio":null}"#);
        assert_eq!(serde_json::to_string(&set).unwrap(), r#"{"bio":"x"}"#);
    }

    #[test]
    fn conversions_and_apply() {
        assert_eq!(Maybe::<String>::set("a"), Maybe::Set("a".to_string()));
        assert_eq!(Maybe::<u8>::nullable(None), Maybe::Null);
        assert_eq!(Maybe::<u8>::nullable(Some(1)), Maybe::Set(1));
        assert_eq!(Maybe::<u8>::from(None::<Option<u8>>), Maybe::Unset);
        assert_eq!(Maybe::<u8>::from(Some(None::<u8>)), Maybe::Null);
        assert_eq!(Maybe::Set(1).into_nested(), Some(Some(1)));

        let mut target = Some(1);
        Maybe::Unset.apply_to(&mut target);
        assert_eq!(target, Some(1));
        Maybe::Set(2).apply_to(&mut target);
        assert_eq!(target, Some(2));
        Maybe::Null.apply_to(&mut target);
        assert_eq!(target, None);
        assert_eq!(Maybe::Set(2).map(|n| n * 2).into_value(), Some(4));
    }
}
