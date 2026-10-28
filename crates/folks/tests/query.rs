use folks::garde::Validate as _;
use folks::{Entity, Filter, SortOrder};
use serde_json::{Value, json};

#[derive(Entity)]
#[entity(matches, query(paginate))]
#[serde(rename_all = "snake_case")]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(readonly, query(range), sortable)]
    created_at: i64,
    #[entity(query(contains), validate(length(max = 12), ascii))]
    name: String,
}

#[test]
fn range_contains_sort_and_page_are_generated() {
    let query: UserQueryRequest = serde_json::from_value(json!({
        "created_at_from": 10,
        "created_at_to": 20,
        "name_contains": "ali",
        "sort": "created_at",
        "order": "desc",
        "page": 2,
        "per_page": 500
    }))
    .unwrap();
    assert!(query.validate().is_ok());
    assert_eq!(query.order, Some(SortOrder::Desc));
    assert_eq!(query.page_request().page, 2);
    assert_eq!(query.page_request().limit(), 100);
    assert!(
        query.matches(&User {
            id: 1,
            created_at: 15,
            name: "alice".into()
        }),
        "{query:?}"
    );
    assert!(!query.matches(&User {
        id: 2,
        created_at: 25,
        name: "alice".into()
    }));
    assert!(!query.matches(&User {
        id: 3,
        created_at: 15,
        name: "Bob".into()
    }));
}

#[test]
fn default_query_and_sort_serde_names() {
    let query: UserQueryRequest = serde_json::from_value(json!({})).unwrap();
    assert_eq!(query.page_request().page, 1);
    assert_eq!(
        query.page_request().per_page,
        folks::core::page::DEFAULT_PER_PAGE
    );
    assert_eq!(
        serde_json::to_value(UserSortField::CreatedAt).unwrap(),
        Value::String("created_at".into())
    );
}

#[test]
fn equality_can_be_combined_with_range() {
    #[derive(Entity)]
    #[entity(matches)]
    struct Event {
        #[entity(id)]
        id: u64,
        #[entity(query(eq, range))]
        score: i32,
    }
    let query = EventQueryRequest {
        id: None,
        score: Some(5),
        score_from: Some(3),
        score_to: None,
    };
    assert!(query.matches(&Event { id: 1, score: 5 }));
    assert!(!query.matches(&Event { id: 2, score: 6 }));
}

#[test]
fn generated_range_fields_run_validation_rules() {
    #[derive(Entity)]
    struct Measurement {
        #[entity(id)]
        id: u64,
        #[entity(query(range), validate(range(min = 0, max = 10)))]
        value: i32,
    }
    let _entity = Measurement { id: 0, value: 0 };
    assert_eq!(_entity.value, 0);
    let query = MeasurementQueryRequest {
        id: None,
        value_from: Some(-1),
        value_to: Some(11),
    };
    assert!(query.validate().is_err());
}

#[test]
fn derived_filters_match_optional_source_values() {
    #[derive(Entity)]
    #[entity(matches)]
    struct OptionalValue {
        #[entity(id)]
        id: u64,
        #[entity(query(range))]
        score: Option<i32>,
        #[entity(query(contains))]
        label: Option<String>,
    }
    let query = OptionalValueQueryRequest {
        id: None,
        score_from: Some(3),
        score_to: Some(8),
        label_contains: Some("match".into()),
    };
    assert!(query.matches(&OptionalValue {
        id: 1,
        score: Some(5),
        label: Some("a match here".into()),
    }));
    assert!(!query.matches(&OptionalValue {
        id: 2,
        score: None,
        label: Some("a match here".into()),
    }));
}
