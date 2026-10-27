//! A separate crate that depends on `folks` under another name (`my_folks`)
//! and has no direct `serde` / `serde_with` / `garde` dependency.

use folks_consumer::*;
use my_folks::garde::Validate;
use my_folks::{Diff, Error, Filter, Maybe, Patch};

fn customer() -> Customer {
    Customer {
        id: 1,
        name: "Ann".into(),
        bio: Some("hi".into()),
        tags: vec!["a".into()],
        age: 30,
        created_at: "2026".into(),
        secret: "s".into(),
    }
}

#[test]
fn derive_works_through_a_renamed_dependency() {
    let c = CustomerCreateRequest::new("Ann", vec!["a".to_string()], 30u8).with_bio("hi");
    assert!(c.validate().is_ok());

    let bad = CustomerCreateRequest::new("", vec![String::new()], 200u8);
    match bad.validate().map_err(Error::from) {
        Err(Error::Invalid(errors)) => {
            let fields: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
            assert_eq!(fields, ["name", "tags[0]", "age"]);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn patch_diff_and_filter_work_through_the_rename() {
    let old = customer();
    let mut new = customer();
    new.name = "Bea".into();
    new.bio = None;

    let d = CustomerUpdateRequest::diff(&old, &new);
    assert_eq!(d.name.as_deref(), Some("Bea"));
    assert_eq!(d.bio, Maybe::Null);

    let mut target = customer();
    d.apply_to(&mut target);
    assert_eq!((target.name.as_str(), target.bio), ("Bea", None));

    assert!(
        CustomerQueryRequest::default()
            .with_name("Ann")
            .matches(&old)
    );
    // `strict` + `rename_all` apply to requests.
    assert!(
        serde_json::from_str::<CustomerCreateRequest>(r#"{"name":"a","tags":[],"age":1,"x":1}"#)
            .is_err()
    );
    let u: CustomerUpdateRequest = serde_json::from_str(r#"{"bio":null}"#).unwrap();
    assert_eq!(u.bio, Maybe::Null);
    let r: CustomerResponse = old.into();
    assert_eq!(serde_json::to_value(&r).unwrap()["createdAt"], "2026");
}
