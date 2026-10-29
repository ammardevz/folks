use folks::Entity;

#[derive(Debug, PartialEq)]
struct StoredName(String);

impl From<StoredName> for String {
    fn from(value: StoredName) -> Self {
        value.0
    }
}

struct DatabaseUser {
    id: u32,
    name: StoredName,
    nickname: Option<StoredName>,
}

struct LegacyUser {
    id: u8,
    name: String,
    nickname: Option<StoredName>,
}

struct MappedModel {
    id: u64,
    email_address: String,
    name: String,
    disabled_at: Option<u64>,
}

fn is_set<T>(value: Option<T>) -> bool {
    value.is_some()
}

fn normalize_name(value: String) -> String {
    value.trim().to_owned()
}

#[derive(Entity)]
#[entity(from(MappedModel))]
struct MappedUser {
    #[entity(id)]
    id: u64,
    #[entity(map(from = "email_address"))]
    email: String,
    #[entity(map(with = "normalize_name"))]
    name: String,
    #[entity(map(from = "disabled_at", with = "is_set"))]
    disabled: bool,
}

#[test]
fn source_fields_support_renaming_conversion_and_combined_mapping() {
    let response = MappedUserResponse::from(MappedModel {
        id: 5,
        email_address: "a@example.test".into(),
        name: "  Ada  ".into(),
        disabled_at: Some(10),
    });
    assert_eq!(response.email, "a@example.test");
    assert_eq!(response.name, "Ada");
    assert!(response.disabled);

    let response = MappedUserResponse::from(MappedModel {
        id: 6,
        email_address: "b@example.test".into(),
        name: "Grace".into(),
        disabled_at: None,
    });
    assert!(!response.disabled);
}

#[derive(Entity)]
#[entity(from(DatabaseUser))]
#[entity(from(LegacyUser))]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(response(rename = "displayName"))]
    name: String,
    nickname: Option<String>,
}

#[test]
fn response_converts_from_multiple_source_types() {
    let response = UserResponse::from(DatabaseUser {
        id: 7,
        name: StoredName("Ada".into()),
        nickname: Some(StoredName("A".into())),
    });
    assert_eq!(response.id, 7);
    assert_eq!(response.name, "Ada");
    assert_eq!(response.nickname.as_deref(), Some("A"));
    assert_eq!(
        serde_json::to_value(&response).unwrap()["displayName"],
        "Ada"
    );

    let response = UserResponse::from(LegacyUser {
        id: 8,
        name: "Grace".into(),
        nickname: Some(StoredName("G".into())),
    });
    assert_eq!(response.id, 8);
    assert_eq!(response.name, "Grace");
    assert_eq!(response.nickname.as_deref(), Some("G"));
}

struct GenericModel<T> {
    id: u64,
    value: T,
}

#[derive(Entity)]
#[entity(from(GenericModel<T>))]
struct GenericEntity<T> {
    #[entity(id)]
    id: u64,
    value: T,
}

#[test]
fn source_conversions_keep_used_generics() {
    let response: GenericEntityResponse<String> = GenericModel {
        id: 3,
        value: "generic".into(),
    }
    .into();
    assert_eq!((response.id, response.value.as_str()), (3, "generic"));
}
