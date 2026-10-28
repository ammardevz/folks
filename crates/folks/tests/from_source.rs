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
