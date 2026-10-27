//! The request types are plain `garde::Validate` implementations.

use folks::garde::Validate;
use folks::{Entity, Error, FieldError};

#[derive(Entity)]
#[serde(rename_all = "camelCase")]
#[entity(update(rename_all = "kebab-case"))]
pub struct Account {
    #[entity(id)]
    pub id: u32,
    #[entity(validate(length(min = 1)))]
    pub display_name: String,
    #[serde(rename = "mail")]
    #[entity(validate(length(min = 3)))]
    pub email_address: String,
    #[entity(validate(each(length(min = 2))))]
    pub phone_numbers: Vec<String>,
    #[entity(validate(custom(even)))]
    pub lucky_number: u8,
}

fn even(n: &u8) -> Result<(), String> {
    if n.is_multiple_of(2) {
        Ok(())
    } else {
        Err(format!("{n} is odd"))
    }
}

fn bad_create() -> AccountCreateRequest {
    AccountCreateRequest {
        display_name: String::new(),
        email_address: "a".into(),
        phone_numbers: vec!["12".into(), "1".into()],
        lucky_number: 3,
    }
}

fn paths(report: &folks::garde::Report) -> Vec<String> {
    report.iter().map(|(p, _)| p.to_string()).collect()
}

#[test]
fn paths_use_serialized_names() {
    let report = bad_create().validate().unwrap_err();
    assert_eq!(
        paths(&report),
        ["displayName", "mail", "phoneNumbers[1]", "luckyNumber"]
    );
}

#[test]
fn per_type_rename_all_changes_the_paths() {
    let update = AccountUpdateRequest {
        display_name: Some(String::new()),
        ..Default::default()
    };
    assert_eq!(paths(&update.validate().unwrap_err()), ["display-name"]);
}

#[test]
fn custom_errors_keep_their_message() {
    let report = bad_create().validate().unwrap_err();
    let (_, e) = report.iter().last().unwrap();
    assert_eq!(e.message(), "3 is odd");
}

#[test]
fn empty_update_is_a_root_level_error() {
    let report = AccountUpdateRequest::default().validate().unwrap_err();
    let entries: Vec<_> = report.iter().collect();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].0.is_empty());
    assert!(
        entries[0]
            .1
            .message()
            .contains("must set at least one field")
    );
    assert!(matches!(Error::from(report), Error::ValidationError(_)));
}

#[test]
fn report_converts_to_folks_error() {
    let err = Error::from(bad_create().validate().unwrap_err());
    let fields: Vec<&str> = err.fields().iter().map(|f| f.field.as_str()).collect();
    assert_eq!(
        fields,
        ["displayName", "mail", "phoneNumbers[1]", "luckyNumber"]
    );
    assert_eq!(err.fields()[3], FieldError::new("luckyNumber", "3 is odd"));
}

#[test]
fn question_mark_works_in_handlers() {
    fn handler(r: &AccountCreateRequest) -> Result<(), Error> {
        r.validate()?;
        Ok(())
    }
    assert!(matches!(handler(&bad_create()), Err(Error::Invalid(_))));
}
