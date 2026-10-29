//! Per-type field overrides: `create(ty = ..)`, `response(rename = ..)`, ...

use folks::garde::Validate;
use folks::{Entity, Patch};
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub struct Email(pub String);

impl From<String> for Email {
    fn from(s: String) -> Self {
        Email(s)
    }
}

impl From<Email> for String {
    fn from(e: Email) -> Self {
        e.0
    }
}

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(patch)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    #[entity(id)]
    pub id: u32,

    /// Stored as a newtype, exposed as plain text everywhere.
    #[entity(
        response(ty = String),
        create(ty = String),
        update(ty = String),
        query(ty = String),
        validate(length(min = 3))
    )]
    pub email: Email,

    /// Optional newtype: `Option` converts through its inner value.
    #[entity(
        response(ty = Option<String>),
        create(ty = Option<String>),
        update(ty = Option<String>)
    )]
    pub backup_email: Option<Email>,

    /// Stored narrow, exposed wide in the response, accepted narrow in updates.
    #[entity(response(ty = u64), update(ty = u8, rename = "qty"), create)]
    pub visits: u32,

    #[serde(rename = "joined")]
    #[entity(
        response(rename = "memberSince"),
        create(rename = "joined_on"),
        validate(length(min = 4))
    )]
    pub joined_at: String,
}

fn member() -> Member {
    Member {
        id: 7,
        email: Email("ann@example.com".into()),
        backup_email: Some(Email("ann@home".into())),
        visits: 5,
        joined_at: "2020".into(),
    }
}

#[test]
fn response_converts_overridden_fields_and_uses_its_own_names() {
    let response: MemberResponse = member().into();
    assert_eq!(
        serde_json::to_value(&response).unwrap(),
        json!({
            "id": 7,
            "email": "ann@example.com",
            "backupEmail": "ann@home",
            "visits": 5,
            "memberSince": "2020",
        })
    );
    let response: MemberResponse = Member {
        backup_email: None,
        ..member()
    }
    .into();
    assert_eq!(response.backup_email, None);
}

#[test]
fn create_uses_the_override_type_and_name() {
    let create: MemberCreateRequest = serde_json::from_value(json!({
        "email": "abc",
        "visits": 2,
        "joined_on": "2021",
    }))
    .unwrap();
    assert_eq!(create.email, "abc");
    assert_eq!(create.visits, 2u32); // bare `create`: declared type
    assert_eq!(create.backup_email, None);
    assert_eq!(create.joined_at, "2021");

    // `new` takes the override type.
    let built = MemberCreateRequest::new("abc", 1u32, "2021");
    assert!(built.validate().is_ok());
}

#[test]
fn update_applies_through_into() {
    let update: MemberUpdateRequest = serde_json::from_value(json!({
        "email": "new@example.com",
        "backupEmail": "b@x",
        "qty": 9,
    }))
    .unwrap();
    assert_eq!(update.visits, Some(9u8));
    let mut m = member();
    update.apply_to(&mut m);
    assert_eq!(m.email, Email("new@example.com".into()));
    assert_eq!(m.backup_email, Some(Email("b@x".into())));
    assert_eq!(m.visits, 9);
    assert_eq!(m.joined_at, "2020");
}

#[test]
fn query_uses_the_override_type() {
    let q = MemberQueryRequest {
        id: None,
        email: Some("x".to_string()),
    };
    assert_eq!(q.email.as_deref(), Some("x"));
}

#[test]
fn rules_run_against_overrides_and_report_per_type_names() {
    let bad = MemberCreateRequest::new("ab", 1u32, "20");
    let report = bad.validate().unwrap_err();
    let paths: Vec<String> = report.iter().map(|(p, _)| p.to_string()).collect();
    assert_eq!(paths, ["email", "joined_on"]);

    let update = MemberUpdateRequest::default().with_email("ab");
    let report = update.validate().unwrap_err();
    let paths: Vec<String> = report.iter().map(|(p, _)| p.to_string()).collect();
    assert_eq!(paths, ["email"]);
}

#[test]
fn field_is_only_in_the_listed_types() {
    // Struct literals fail to compile if a field is missing or extra.
    let _ = MemberResponse {
        id: 1,
        email: String::new(),
        backup_email: None,
        visits: 1,
        joined_at: String::new(),
    };
    let _ = MemberUpdateRequest {
        email: None,
        backup_email: None,
        visits: None,
    };
}

#[derive(Entity)]
pub struct ValidationOverrideTypeCheck {
    #[entity(id)]
    pub id: u8,
    #[entity(
        response,
        create(ty = u32, validate(range(min = 4))),
        update,
        query,
        validate(length(min = 3))
    )]
    pub value: String,
}

#[test]
fn validation_override_skips_shared_type_check_only_for_its_target() {
    assert!(
        ValidationOverrideTypeCheckCreateRequest { value: 4 }
            .validate()
            .is_ok()
    );
    assert!(
        ValidationOverrideTypeCheckCreateRequest { value: 3 }
            .validate()
            .is_err()
    );

    assert!(
        ValidationOverrideTypeCheckUpdateRequest::default()
            .with_value("abc")
            .validate()
            .is_ok()
    );
    assert!(
        ValidationOverrideTypeCheckUpdateRequest::default()
            .with_value("ab")
            .validate()
            .is_err()
    );

    assert!(
        ValidationOverrideTypeCheckQueryRequest {
            value: Some("abc".into()),
            ..Default::default()
        }
        .validate()
        .is_ok()
    );
    assert!(
        ValidationOverrideTypeCheckQueryRequest {
            value: Some("a".into()),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}
