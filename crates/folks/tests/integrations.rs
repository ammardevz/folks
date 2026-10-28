//! Optional integrations; run with `--features schemars,utoipa`.
#![cfg(any(feature = "schemars", feature = "utoipa"))]

use folks::Entity;

#[cfg(feature = "schemars")]
mod schemars_tests {
    use super::*;

    #[derive(Entity)]
    #[entity(schemars, patch, query(paginate))]
    pub struct Book {
        #[entity(id)]
        pub id: u32,
        #[entity(sortable)]
        pub title: String,
        pub subtitle: Option<String>,
    }

    #[test]
    fn generated_types_implement_json_schema() {
        let schema = folks::schemars::schema_for!(BookCreateRequest);
        let json = serde_json::to_value(&schema).unwrap();
        assert!(json["properties"]["title"].is_object(), "{json}");
        assert!(json["properties"].get("id").is_none());
        let sort = folks::schemars::schema_for!(BookSortField);
        assert!(serde_json::to_value(sort).unwrap().is_object());

        // Even the merge-patch update type (`Option<Option<T>>`) has a schema.
        let update = serde_json::to_value(folks::schemars::schema_for!(BookUpdateRequest)).unwrap();
        assert!(update["properties"]["subtitle"].is_object(), "{update}");
    }
}

#[cfg(feature = "utoipa")]
mod utoipa_tests {
    use super::*;

    #[derive(Entity)]
    #[entity(utoipa, query(paginate))]
    pub struct Movie {
        #[entity(id)]
        pub id: u32,
        #[entity(sortable)]
        pub title: String,
        #[entity(clearable)]
        pub note: Option<String>,
    }

    #[test]
    fn generated_types_implement_to_schema() {
        use utoipa::ToSchema;
        assert_eq!(MovieResponse::name(), "MovieResponse");
        assert_eq!(MovieCreateRequest::name(), "MovieCreateRequest");
        // The clearable update field (`Maybe<String>`) is described as nullable.
        assert_eq!(MovieUpdateRequest::name(), "MovieUpdateRequest");
        assert_eq!(MovieSortField::name(), "MovieSortField");
    }
}
