//! Sort direction for generated query requests.

use serde::{Deserialize, Serialize};

/// Sort direction, the `order` of a query request (`?order=desc`).
///
/// `Asc` is the default, so `query.order.unwrap_or_default()` is always a
/// direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Smallest first.
    #[default]
    Asc,
    /// Largest first.
    Desc,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_lowercase_and_defaults_to_asc() {
        assert_eq!(serde_json::to_string(&SortOrder::Desc).unwrap(), "\"desc\"");
        assert_eq!(
            serde_json::from_str::<SortOrder>("\"asc\"").unwrap(),
            SortOrder::Asc
        );
        assert_eq!(SortOrder::default(), SortOrder::Asc);
    }
}
