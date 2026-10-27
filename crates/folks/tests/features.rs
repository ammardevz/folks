//! `From`, `Default` + setters, `apply_to`, extra derives, shorthands, `Page`.

use folks::{Entity, Page, PageRequest, Patch};
use uuid::Uuid;

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(derive(PartialEq, Eq))]
pub struct Article {
    #[entity(id)]
    pub id: u32,
    pub title: String,
    pub subtitle: Option<String>,
    #[entity(readonly)]
    pub created_at: String,
    #[entity(immutable)]
    pub slug: String,
    #[entity(skip)]
    pub internal: u8,
}

fn article() -> Article {
    Article {
        id: 1,
        title: "T".into(),
        subtitle: None,
        created_at: "2026".into(),
        slug: "t".into(),
        internal: 9,
    }
}

#[test]
fn from_entity_builds_the_response() {
    let r: ArticleResponse = article().into();
    assert_eq!(
        r,
        ArticleResponse {
            id: 1,
            title: "T".into(),
            subtitle: None,
            created_at: "2026".into(),
            slug: "t".into(),
        }
    );
}

#[test]
fn readonly_and_immutable_shorthands() {
    // readonly: response + query only; immutable: response + create + query.
    let _ = ArticleCreateRequest {
        title: "t".into(),
        subtitle: None,
        slug: "s".into(),
    };
    let _ = ArticleUpdateRequest {
        title: None,
        subtitle: None,
    };
    let _ = ArticleQueryRequest {
        id: None,
        title: None,
        subtitle: None,
        created_at: Some("x".into()),
        slug: None,
    };
}

#[test]
fn extra_derives_are_applied_to_every_type() {
    // `PartialEq`/`Eq` come from `#[entity(derive(..))]`.
    assert_eq!(
        ArticleDeleteRequest { id: 1 },
        ArticleDeleteRequest { id: 1 }
    );
    assert_eq!(
        ArticleUpdateRequest::default(),
        ArticleUpdateRequest::default()
    );
}

#[test]
fn default_and_fluent_setters() {
    let q = ArticleQueryRequest::default()
        .with_title("hello")
        .with_subtitle("sub");
    assert_eq!(q.title.as_deref(), Some("hello"));
    assert_eq!(q.subtitle.as_deref(), Some("sub"));
    assert!(q.id.is_none() && !q.is_empty());
    assert!(ArticleQueryRequest::default().is_empty());

    // `Into` makes `&str` work for `String`; numbers convert too.
    assert_eq!(ArticleQueryRequest::default().with_id(7u8).id, Some(7));
}

#[test]
fn apply_to_only_touches_supplied_fields() {
    let mut a = article();
    ArticleUpdateRequest::default()
        .with_title("New")
        .apply_to(&mut a);
    assert_eq!((a.title.as_str(), a.subtitle.as_deref()), ("New", None));
    assert_eq!((a.created_at.as_str(), a.internal), ("2026", 9));

    // A field declared `Option<T>` is set to `Some(v)`.
    ArticleUpdateRequest::default()
        .with_subtitle("sub")
        .apply_to(&mut a);
    assert_eq!(a.subtitle.as_deref(), Some("sub"));

    // Nothing supplied: nothing changes.
    let before = a.clone();
    ArticleUpdateRequest::default().apply_to(&mut a);
    assert_eq!(a, before);
}

// ------------------------------------------------------------ generics

#[derive(Entity)]
pub struct Boxed<'a, T, U: Clone>
where
    T: Clone,
{
    #[entity(id)]
    pub id: u32,
    pub value: T,
    pub label: &'a str,
    #[entity(skip)]
    pub extra: Option<U>,
}

#[test]
fn generic_entities_get_from_and_apply_to() {
    let mut b = Boxed {
        id: 1,
        value: 5u8,
        label: "a",
        extra: Some(1.5f32),
    };
    let update = BoxedUpdateRequest::default()
        .with_value(6u8)
        .with_label("b");
    update.apply_to(&mut b); // `U` is inferred as a method generic
    assert_eq!((b.value, b.label), (6, "b"));

    let r: BoxedResponse<'_, u8> = b.into();
    assert_eq!((r.id, r.value, r.label), (1, 6, "b"));
}

// ------------------------------------------------------------- id-only

#[derive(Entity)]
pub struct OnlyId {
    #[entity(id)]
    pub id: Uuid,
}

#[test]
fn id_only_entity_still_gets_everything() {
    let id = Uuid::nil();
    let r: OnlyIdResponse = OnlyId { id }.into();
    assert_eq!(r.id, id);
    let mut e = OnlyId { id };
    OnlyIdUpdateRequest::default().apply_to(&mut e);
    assert!(OnlyIdUpdateRequest::default().is_empty());
    assert_eq!(OnlyIdQueryRequest::default().with_id(id).id, Some(id));
}

// ---------------------------------------------------------------- Page

#[test]
fn page_maps_entities_to_responses() {
    let req = PageRequest::new(1, 10);
    let page = Page::new(vec![article()], 1, &req).map(ArticleResponse::from);
    assert_eq!(page.items[0].id, 1);
    let json = serde_json::to_value(&page).unwrap();
    assert_eq!(json["total"], 1);
    assert_eq!(json["items"][0]["title"], "T");
}
