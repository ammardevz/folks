//! `#[entity(secret)]`: redacted in `Debug`, never in response or query.

use folks::Entity;

const PASSWORD: &str = "hunter2-very-secret";

#[derive(Entity)]
pub struct Account {
    #[entity(id)]
    pub id: u32,
    pub name: String,
    #[entity(secret)]
    pub password: String,
    #[entity(secret, create)]
    pub pin: Option<String>,
    pub r#type: String,
}

#[derive(Entity)]
pub struct Wrapper<T> {
    #[entity(id)]
    pub id: u32,
    #[entity(secret)]
    pub token: String,
    pub extra: T,
}

#[test]
fn secret_fields_are_only_in_requests_that_accept_them() {
    let _ = AccountCreateRequest {
        name: "a".into(),
        password: PASSWORD.into(),
        pin: Some("1234".into()),
        r#type: "t".into(),
    };
    let _ = AccountUpdateRequest {
        name: None,
        password: Some(PASSWORD.into()),
        r#type: None,
    };
    // Struct literals fail to compile if these carried a secret.
    let _ = AccountResponse {
        id: 1,
        name: "a".into(),
        r#type: "t".into(),
    };
    let _ = AccountQueryRequest {
        id: None,
        name: None,
        r#type: None,
    };
}

#[test]
fn debug_of_create_never_contains_the_secret() {
    let create = AccountCreateRequest {
        name: "ann".into(),
        password: PASSWORD.into(),
        pin: Some("1234".into()),
        r#type: "t".into(),
    };
    for text in [format!("{create:?}"), format!("{create:#?}")] {
        assert!(!text.contains(PASSWORD), "{text}");
        assert!(!text.contains("1234"), "{text}");
        assert!(text.contains("password: ***"), "{text}");
        assert!(text.contains("name: \"ann\""), "{text}");
        assert!(text.contains("type: \"t\""), "{text}");
    }
    assert!(format!("{create:?}").starts_with("AccountCreateRequest {"));
}

#[test]
fn debug_of_update_never_contains_the_secret() {
    let update = AccountUpdateRequest {
        name: Some("ann".into()),
        password: Some(PASSWORD.into()),
        r#type: None,
    };
    let text = format!("{update:?}");
    assert!(!text.contains(PASSWORD), "{text}");
    assert!(text.contains("password: ***"), "{text}");
}

#[test]
fn types_without_secrets_keep_the_derived_debug() {
    let response = AccountResponse {
        id: 1,
        name: "a".into(),
        r#type: "t".into(),
    };
    assert_eq!(
        format!("{response:?}"),
        "AccountResponse { id: 1, name: \"a\", type: \"t\" }"
    );
}

#[test]
fn redacted_debug_works_for_generic_entities() {
    let create = WrapperCreateRequest {
        token: PASSWORD.into(),
        extra: 5u8,
    };
    let text = format!("{create:?}");
    assert!(!text.contains(PASSWORD), "{text}");
    assert!(text.contains("extra: 5"), "{text}");
}

#[test]
fn secrets_still_serialize_and_deserialize() {
    let create: AccountCreateRequest =
        serde_json::from_str(r#"{"name":"a","password":"p","type":"t"}"#).unwrap();
    assert_eq!(create.password, "p");
}
