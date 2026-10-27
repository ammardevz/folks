//! Every validation rule, with the declared-type semantics.

use folks::prelude::*;
use folks::{Error, FieldError};

fn fields(r: Result<(), folks::garde::Report>) -> Vec<(String, String)> {
    match r.map_err(Error::from) {
        Err(Error::Invalid(e)) => e.into_iter().map(|e| (e.field, e.message)).collect(),
        other => panic!("expected field errors, got {other:?}"),
    }
}

fn short_list(v: &[String]) -> Result<(), String> {
    if v.len() <= 2 {
        Ok(())
    } else {
        Err(format!("{} items, at most 2", v.len()))
    }
}

fn names(r: Result<(), folks::garde::Report>) -> Vec<String> {
    fields(r).into_iter().map(|(f, _)| f).collect()
}

fn digits_only(v: &str) -> Result<(), String> {
    if v.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(format!("`{v}` is not numeric"))
    }
}

#[derive(Entity)]
pub struct Everything {
    #[entity(id)]
    pub id: u32,
    #[entity(validate(length(min = 2, max = 4)))]
    pub short: String,
    #[entity(validate(length(equal = 3)))]
    pub exact: String,
    #[entity(validate(length(chars, max = 2)))]
    pub two_chars: String,
    #[entity(validate(length(bytes, max = 2)))]
    pub two_bytes: String,
    #[entity(validate(range(min = 1, max = 10)))]
    pub small: u8,
    #[entity(validate(range(min = -5, max = 5)))]
    pub signed: i32,
    #[entity(validate(range(min = 0.5, max = 1)))]
    pub ratio: f64,
    #[entity(validate(ascii, alphanumeric))]
    pub code: String,
    #[entity(validate(contains("@"), prefix("a"), suffix("z")))]
    pub token: String,
    #[entity(validate(length(min = 1), custom(digits_only)))]
    pub digits: String,
    #[entity(validate(required, length(min = 1)))]
    pub must: Option<String>,
    #[entity(validate(length(max = 2)))]
    pub tags_len: Vec<u8>,
    #[entity(validate(each(length(max = 2))))]
    pub tags: Vec<String>,
    #[entity(validate(each(each(range(max = 9)))))]
    pub grid: Vec<Vec<u8>>,
    // `custom` can take a slice for a `Vec`.
    #[entity(validate(custom(short_list)))]
    pub list: Vec<String>,
}

fn good() -> EverythingCreateRequest {
    EverythingCreateRequest {
        short: "abc".into(),
        exact: "abc".into(),
        two_chars: "éé".into(),
        two_bytes: "ab".into(),
        small: 5,
        signed: -5,
        ratio: 0.75,
        code: "abc123".into(),
        token: "a@z".into(),
        digits: "123".into(),
        must: Some("x".into()),
        tags_len: vec![1],
        tags: vec!["ab".into()],
        grid: vec![vec![1, 2], vec![9]],
        list: vec![],
    }
}

#[test]
fn a_valid_request_passes() {
    assert!(good().validate().is_ok());
}

#[test]
fn length_modes_and_bounds() {
    let mut r = good();
    r.short = "a".into();
    assert_eq!(
        fields(r.validate()),
        [("short".into(), "length is lower than 2".into())]
    );
    let mut r = good();
    r.short = "abcde".into();
    assert_eq!(names(r.validate()), ["short"]);
    let mut r = good();
    r.exact = "ab".into();
    assert_eq!(names(r.validate()), ["exact"]);

    // `chars` counts characters, `bytes` counts bytes: "éé" is 2 chars, 4 bytes.
    let mut r = good();
    r.two_bytes = "éé".into();
    assert_eq!(names(r.validate()), ["two_bytes"]);
    let mut r = good();
    r.two_chars = "ééé".into();
    assert_eq!(names(r.validate()), ["two_chars"]);

    let mut r = good();
    r.tags_len = vec![1, 2, 3];
    assert_eq!(names(r.validate()), ["tags_len"]);
}

#[test]
fn range_on_ints_signed_ints_and_floats() {
    let mut r = good();
    r.small = 0;
    assert_eq!(names(r.validate()), ["small"]);
    r.small = 11;
    assert_eq!(names(r.validate()), ["small"]);

    let mut r = good();
    r.signed = -6;
    assert_eq!(names(r.validate()), ["signed"]);
    r.signed = 6;
    assert_eq!(names(r.validate()), ["signed"]);

    // The float field accepts the integer bound `1`.
    let mut r = good();
    r.ratio = 0.4;
    assert_eq!(names(r.validate()), ["ratio"]);
    r.ratio = 1.5;
    assert_eq!(names(r.validate()), ["ratio"]);
}

#[test]
fn text_rules() {
    let mut r = good();
    r.code = "añb".into();
    assert_eq!(names(r.validate()), ["code"]); // not ascii (but Unicode-alphanumeric)
    r.code = "a-b".into();
    assert_eq!(names(r.validate()), ["code"]); // ascii, but not alphanumeric
    r.code = "a-ñ".into();
    assert_eq!(names(r.validate()), ["code", "code"]); // fails both
    let mut r = good();
    r.token = "a-z".into();
    assert_eq!(names(r.validate()), ["token"]); // missing "@"
    r.token = "b@z".into();
    assert_eq!(names(r.validate()), ["token"]); // wrong prefix
    r.token = "a@y".into();
    assert_eq!(names(r.validate()), ["token"]); // wrong suffix
}

#[test]
fn rules_on_one_field_all_run_and_report() {
    let mut r = good();
    r.digits = "".into();
    // length(min = 1) passes through custom: "" is vacuously numeric.
    assert_eq!(names(r.validate()), ["digits"]);
    r.digits = "12a".into();
    assert_eq!(
        fields(r.validate()),
        [("digits".into(), "`12a` is not numeric".into())]
    );
}

#[test]
fn required_is_enforced_for_declared_option_in_create() {
    let mut r = good();
    r.must = None;
    assert_eq!(fields(r.validate()), [("must".into(), "not set".into())]);
    r.must = Some(String::new());
    assert_eq!(names(r.validate()), ["must"]); // present, but length(min = 1) fails
}

#[test]
fn custom_takes_the_deref_target() {
    let mut r = good();
    r.list = vec!["a".into(), "b".into(), "c".into()];
    assert_eq!(
        fields(r.validate()),
        [("list".into(), "3 items, at most 2".into())]
    );
}

#[test]
fn each_reports_the_element_index() {
    let mut r = good();
    r.tags = vec!["ok".into(), "toolong".into(), "x".into(), "alsolong".into()];
    assert_eq!(names(r.validate()), ["tags[1]", "tags[3]"]);

    let mut r = good();
    r.grid = vec![vec![1], vec![2, 10]];
    assert_eq!(names(r.validate()), ["grid[1][1]"]);
}

#[test]
fn rules_hold_in_update_and_query_for_present_values_only() {
    let update = EverythingUpdateRequest::default().with_short("a");
    assert_eq!(names(update.validate()), ["short"]);
    let ok = EverythingUpdateRequest::default()
        .with_short("abc")
        .with_small(3u8);
    assert!(ok.validate().is_ok());
    // `required` is a create-only rule: absent in an update is fine.
    assert!(
        EverythingUpdateRequest::default()
            .with_exact("abc")
            .validate()
            .is_ok()
    );

    assert!(EverythingQueryRequest::default().validate().is_ok());
    assert_eq!(
        names(
            EverythingQueryRequest::default()
                .with_small(99u8)
                .validate()
        ),
        ["small"]
    );
    // Collections are checked in update requests as well.
    let tags = EverythingUpdateRequest::default().with_tags(vec!["toolong".to_string()]);
    assert_eq!(names(tags.validate()), ["tags[0]"]);
}

#[test]
fn field_error_is_serializable() {
    let e = FieldError::new("name", "bad");
    assert_eq!(
        serde_json::to_string(&e).unwrap(),
        r#"{"field":"name","message":"bad"}"#
    );
    assert_eq!(e.to_string(), "name: bad");
}

// ---------------------------------------------------- clearable (Maybe) fields

#[derive(Entity)]
pub struct Profile {
    #[entity(id)]
    pub id: u8,
    #[entity(clearable, validate(length(max = 5), custom(digits_only)))]
    pub phone: Option<String>,
}

#[test]
fn rules_reach_into_maybe_and_skip_unset_and_null() {
    assert_eq!(
        names(
            ProfileUpdateRequest::default()
                .with_phone("123456")
                .validate()
        ),
        ["phone"]
    );
    assert_eq!(
        names(ProfileUpdateRequest::default().with_phone("12a").validate()),
        ["phone"]
    );
    assert!(
        ProfileUpdateRequest::default()
            .with_phone("123")
            .validate()
            .is_ok()
    );
    // Null clears the field: nothing to validate.
    assert!(
        ProfileUpdateRequest::default()
            .clear_phone()
            .validate()
            .is_ok()
    );
    // Unset alone is just an empty update.
    assert!(ProfileUpdateRequest::default().validate().is_err());
}

// -------------------------------------------------------------- &str, generics

#[derive(Entity)]
pub struct Borrowed<'a, T> {
    #[entity(id)]
    pub id: u8,
    #[entity(validate(length(min = 1)))]
    pub name: &'a str,
    pub extra: T,
}

#[test]
fn borrowed_str_fields_and_generics() {
    let r = BorrowedCreateRequest {
        name: "",
        extra: 1u8,
    };
    assert_eq!(names(r.validate()), ["name"]);
    assert!(
        BorrowedCreateRequest {
            name: "x",
            extra: 1u8
        }
        .validate()
        .is_ok()
    );
}

// ------------------------------------------------------------- optional rules

#[cfg(feature = "email")]
mod email_rule {
    use super::*;

    #[derive(Entity)]
    #[allow(dead_code)]
    pub struct Contact {
        #[entity(id)]
        pub id: u8,
        #[entity(validate(email))]
        pub mail: String,
    }

    #[test]
    fn email_rule() {
        assert!(
            ContactCreateRequest {
                mail: "a@b.co".into()
            }
            .validate()
            .is_ok()
        );
        assert_eq!(
            names(
                ContactCreateRequest {
                    mail: "nope".into()
                }
                .validate()
            ),
            ["mail"]
        );
    }
}

#[cfg(feature = "url")]
mod url_rule {
    use super::*;

    #[derive(Entity)]
    #[allow(dead_code)]
    pub struct Link {
        #[entity(id)]
        pub id: u8,
        #[entity(validate(url))]
        pub href: String,
    }

    #[test]
    fn url_rule() {
        assert!(
            LinkCreateRequest {
                href: "https://example.com/x".into()
            }
            .validate()
            .is_ok()
        );
        assert_eq!(
            names(
                LinkCreateRequest {
                    href: "not a url".into()
                }
                .validate()
            ),
            ["href"]
        );
    }
}

#[cfg(feature = "pattern")]
mod pattern_rule {
    use super::*;

    #[derive(Entity)]
    #[allow(dead_code)]
    pub struct Slug {
        #[entity(id)]
        pub id: u8,
        #[entity(validate(pattern("^[a-z0-9-]+$")))]
        pub slug: String,
    }

    #[test]
    fn pattern_rule() {
        assert!(
            SlugCreateRequest {
                slug: "my-post-1".into()
            }
            .validate()
            .is_ok()
        );
        assert_eq!(
            names(
                SlugCreateRequest {
                    slug: "My Post".into()
                }
                .validate()
            ),
            ["slug"]
        );
    }
}
