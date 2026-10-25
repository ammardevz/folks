//! Comfort features: `with_*_opt`, `clear_*`, `with_*_nullable`, `new(..)`,
//! `Maybe`, `diff`, `matches`.

use folks::prelude::*;

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(diff, matches)]
pub struct Product {
    #[entity(id)]
    pub id: u32,
    pub name: String,
    #[entity(clearable)]
    pub description: Option<String>,
    pub price: u32,
    pub tag: Option<String>,
}

fn product() -> Product {
    Product {
        id: 1,
        name: "Pen".into(),
        description: Some("blue".into()),
        price: 5,
        tag: None,
    }
}

// ------------------------------------------------------------------ create

#[test]
fn create_has_new_and_setters() {
    let c = ProductCreateRequest::new("Pen", 5u32)
        .with_description("blue")
        .with_tag_opt(None::<&str>);
    assert_eq!(c.name, "Pen");
    assert_eq!(c.price, 5);
    assert_eq!(c.description.as_deref(), Some("blue"));
    assert_eq!(c.tag, None);

    // `_opt` sets only when `Some`.
    let c = ProductCreateRequest::new("Pen", 5u32).with_tag_opt(Some("t"));
    assert_eq!(c.tag.as_deref(), Some("t"));
    // A required field can be replaced after `new`.
    assert_eq!(
        ProductCreateRequest::new("a", 1u32).with_name("b").name,
        "b"
    );
}

// ------------------------------------------------------------------ update

#[test]
fn update_setters_cover_set_opt_clear_nullable() {
    let u = ProductUpdateRequest::default()
        .with_name("Marker")
        .with_price_opt(Some(9u32));
    assert_eq!((u.name.as_deref(), u.price), (Some("Marker"), Some(9)));

    // `None` leaves the field untouched.
    let u = ProductUpdateRequest::default().with_name_opt(None::<&str>);
    assert!(u.name.is_none() && u.is_empty());

    // The three states of a clearable field.
    assert_eq!(ProductUpdateRequest::default().description, Maybe::Unset);
    assert_eq!(
        ProductUpdateRequest::default()
            .with_description("d")
            .description,
        Maybe::Set("d".to_string())
    );
    assert_eq!(
        ProductUpdateRequest::default()
            .clear_description()
            .description,
        Maybe::Null
    );
    assert_eq!(
        ProductUpdateRequest::default()
            .with_description_nullable(None::<&str>)
            .description,
        Maybe::Null
    );
    assert_eq!(
        ProductUpdateRequest::default()
            .with_description_nullable(Some("x"))
            .description,
        Maybe::Set("x".to_string())
    );
    assert_eq!(
        ProductUpdateRequest::default()
            .with_description_opt(None::<&str>)
            .description,
        Maybe::Unset
    );
    // Clearing counts as a change.
    assert!(
        !ProductUpdateRequest::default()
            .clear_description()
            .is_empty()
    );
}

#[test]
fn json_for_the_clearable_field_is_merge_patch() {
    let u = ProductUpdateRequest::default()
        .clear_description()
        .with_price(7u32);
    // Keys follow field declaration order; unset fields are omitted.
    assert_eq!(
        serde_json::to_string(&u).unwrap(),
        r#"{"description":null,"price":7}"#
    );
    let back: ProductUpdateRequest =
        serde_json::from_str(r#"{"description":null,"price":7}"#).unwrap();
    assert_eq!(back.description, Maybe::Null);
    assert_eq!(back.price, Some(7));
}

// ------------------------------------------------------------------- query

#[test]
fn query_setters() {
    let q = ProductQueryRequest::default()
        .with_name("Pen")
        .with_tag_opt(None::<&str>);
    assert_eq!(q.name.as_deref(), Some("Pen"));
    assert!(q.tag.is_none());
}

// -------------------------------------------------------------------- diff

#[test]
fn diff_builds_the_update_between_two_entities() {
    let old = product();
    let mut new = old.clone();
    new.price = 8;
    new.description = None; // clearable: expressed as null
    new.tag = Some("sale".into());

    let d = ProductUpdateRequest::diff(&old, &new);
    assert_eq!(d.name, None);
    assert_eq!(d.price, Some(8));
    assert_eq!(d.description, Maybe::Null);
    assert_eq!(d.tag.as_deref(), Some("sale"));

    // Applying the diff reproduces the new state.
    let mut target = old.clone();
    d.apply_to(&mut target);
    assert_eq!(target, new);

    // Identical entities give an empty update.
    assert!(ProductUpdateRequest::diff(&old, &old).is_empty());
}

// ----------------------------------------------------------------- matches

#[test]
fn query_matches_in_memory() {
    let p = product();
    assert!(ProductQueryRequest::default().matches(&p)); // empty = everything
    assert!(ProductQueryRequest::default().with_name("Pen").matches(&p));
    assert!(!ProductQueryRequest::default().with_name("Ink").matches(&p));
    assert!(
        ProductQueryRequest::default()
            .with_description("blue")
            .matches(&p)
    ); // Option<T> field
    assert!(!ProductQueryRequest::default().with_tag("x").matches(&p));
    assert!(
        ProductQueryRequest::default()
            .with_id(1u32)
            .with_price(5u32)
            .matches(&p)
    );
    assert!(
        !ProductQueryRequest::default()
            .with_id(1u32)
            .with_price(6u32)
            .matches(&p)
    );
}

// ----------------------------------------------------- generics + diff/matches

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(diff, matches)]
pub struct Cell<T> {
    #[entity(id)]
    pub id: u8,
    pub value: T,
}

#[test]
fn diff_and_matches_bound_only_generic_fields() {
    let a = Cell { id: 1, value: 1u8 };
    let b = Cell { id: 1, value: 2u8 };
    let d = CellUpdateRequest::diff(&a, &b);
    assert_eq!(d.value, Some(2));
    assert!(CellQueryRequest::default().with_value(1u8).matches(&a));
}
