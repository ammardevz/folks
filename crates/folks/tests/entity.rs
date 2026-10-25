use folks::core::entity::{Entity as EntityTrait, Validatable};
use folks::core::error::Error;
use folks::{Entity, serde_with::DisplayFromStr};
use uuid::Uuid;

fn assert_assoc<E, Id, Re, Cr, Up, De, Qu>()
where
    E: EntityTrait<
            EntityId = Id,
            EntityResponse = Re,
            EntityCreateRequest = Cr,
            EntityUpdateRequest = Up,
            EntityDeleteRequest = De,
            EntityQueryRequest = Qu,
        >,
{
}

// ---------------------------------------------------------------- basic

#[derive(Entity)]
pub struct User {
    #[entity(id)]
    pub id: Uuid,
    pub name: String,
}

#[test]
fn associated_types_are_wired() {
    assert_assoc::<
        User,
        Uuid,
        UserResponse,
        UserCreateRequest,
        UserUpdateRequest,
        UserDeleteRequest,
        UserQueryRequest,
    >();
}

#[test]
fn every_type_has_exactly_the_documented_fields() {
    // Struct literals fail to compile if a field is missing or extra.
    let id = Uuid::nil();
    let _ = UserResponse {
        id,
        name: "a".into(),
    };
    let _ = UserCreateRequest { name: "a".into() };
    let _ = UserUpdateRequest {
        name: Some("a".into()),
    };
    let _ = UserDeleteRequest { id };
    let _ = UserQueryRequest {
        id: Some(id),
        name: None,
    };
}

#[derive(Entity)]
pub struct BlogPostComment {
    #[entity(id)]
    #[allow(dead_code)]
    id: u32,
}

#[test]
fn multi_word_names() {
    let _: BlogPostCommentResponse = BlogPostCommentResponse { id: 1 };
    let _: BlogPostCommentCreateRequest = BlogPostCommentCreateRequest {};
    let _: BlogPostCommentUpdateRequest = BlogPostCommentUpdateRequest {};
    let _: BlogPostCommentDeleteRequest = BlogPostCommentDeleteRequest { id: 1 };
    let _: BlogPostCommentQueryRequest = BlogPostCommentQueryRequest { id: None };
}

#[test]
fn id_only_entity_semantics() {
    let create = BlogPostCommentCreateRequest {};
    assert!(create.is_empty());
    assert!(create.is_valid());

    // Nothing can be updated, so an update can never be valid.
    let update = BlogPostCommentUpdateRequest {};
    assert!(update.is_empty());
    assert!(update.is_invalid());

    assert!(!BlogPostCommentDeleteRequest { id: 1 }.is_empty());
    assert!(BlogPostCommentQueryRequest { id: None }.is_empty());
    assert!(!BlogPostCommentQueryRequest { id: Some(1) }.is_empty());
}

#[test]
fn private_entity_types_are_private_to_module() {
    // `BlogPostComment` has no `pub`, so generated types inherit that.
    let _ = BlogPostCommentResponse { id: 1 };
}

// ------------------------------------------------------------- Option policy

#[derive(Entity)]
pub struct Profile {
    #[entity(id)]
    pub id: Uuid,
    pub nickname: String,
    pub bio: Option<String>,
}

#[test]
fn nullable_fields_are_not_double_wrapped() {
    // `bio: Option<String>` in update/query is `Option<String>`, not
    // `Option<Option<String>>`: this literal would not compile otherwise.
    let _ = ProfileUpdateRequest {
        nickname: None,
        bio: Some("x".to_string()),
    };
    let _ = ProfileQueryRequest {
        id: None,
        nickname: None,
        bio: Some("x".to_string()),
    };
    let _ = ProfileCreateRequest {
        nickname: "n".into(),
        bio: None,
    };
    let _ = ProfileResponse {
        id: Uuid::nil(),
        nickname: "n".into(),
        bio: None,
    };
}

#[test]
fn update_serialization_omits_unset_fields() {
    let req = ProfileUpdateRequest {
        nickname: Some("n".into()),
        bio: None,
    };
    assert_eq!(serde_json::to_string(&req).unwrap(), r#"{"nickname":"n"}"#);

    let parsed: ProfileUpdateRequest = serde_json::from_str("{}").unwrap();
    assert!(parsed.is_empty());
    let parsed: ProfileUpdateRequest = serde_json::from_str(r#"{"bio":"hi"}"#).unwrap();
    assert_eq!(parsed.bio.as_deref(), Some("hi"));
    assert!(!parsed.is_empty());
}

#[test]
fn create_with_required_field_is_never_empty() {
    let req = ProfileCreateRequest {
        nickname: String::new(),
        bio: None,
    };
    assert!(!req.is_empty());
    assert!(serde_json::from_str::<ProfileCreateRequest>(r#"{"bio":"x"}"#).is_err());
}

// ------------------------------------------------ inclusion/exclusion attrs

#[derive(Entity)]
pub struct Account {
    #[entity(id)]
    pub id: Uuid,
    pub email: String,
    #[entity(skip)]
    pub internal_note: String,
    #[entity(response)]
    pub created_at: String,
    #[entity(response, create)]
    pub role: String,
    #[entity(create, update, delete)]
    pub tenant: String,
}

#[test]
fn custom_inclusion() {
    let _ = AccountResponse {
        id: Uuid::nil(),
        email: "e".into(),
        created_at: "t".into(),
        role: "r".into(),
    };
    let _ = AccountCreateRequest {
        email: "e".into(),
        role: "r".into(),
        tenant: "t".into(),
    };
    let _ = AccountUpdateRequest {
        email: None,
        tenant: None,
    };
    let _ = AccountDeleteRequest {
        id: Uuid::nil(),
        tenant: "t".into(),
    };
    let _ = AccountQueryRequest {
        id: None,
        email: None,
    };
}

#[derive(Entity)]
pub struct IdQueryOnly {
    #[entity(id, query)]
    pub id: u8,
}

#[test]
fn id_with_explicit_list_is_restricted_but_stays_in_delete() {
    let _ = IdQueryOnlyDeleteRequest { id: 1 };
    let _ = IdQueryOnlyQueryRequest { id: Some(1) };
    let _ = IdQueryOnlyResponse {};
}

#[derive(Entity)]
#[entity(name = "Member")]
pub struct RenamedThing {
    #[entity(id)]
    pub id: u8,
}

#[test]
fn name_override_changes_prefix() {
    let _ = MemberResponse { id: 1 };
    let _ = MemberCreateRequest {};
}

// ------------------------------------------------------------- raw idents

#[derive(Entity)]
pub struct Raw {
    #[entity(id)]
    pub r#type: u8,
    pub r#match: String,
}

#[test]
fn raw_identifier_fields() {
    let u = RawUpdateRequest {
        r#match: Some("x".into()),
    };
    assert_eq!(serde_json::to_string(&u).unwrap(), r#"{"match":"x"}"#);
    let _ = RawDeleteRequest { r#type: 1 };
}

// ------------------------------------------------------------------ serde

#[derive(Entity)]
#[serde(rename_all = "camelCase")]
pub struct Gadget {
    #[entity(id)]
    pub gadget_id: u32,
    #[serde(rename = "label")]
    pub display_name: String,
    #[serde(default)]
    pub retry_count: u32,
}

#[test]
fn serde_attributes_propagate_where_valid() {
    let r = GadgetResponse {
        gadget_id: 1,
        display_name: "a".into(),
        retry_count: 2,
    };
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        r#"{"gadgetId":1,"label":"a","retryCount":2}"#
    );

    // `default` is honoured on create ...
    let c: GadgetCreateRequest = serde_json::from_str(r#"{"label":"a"}"#).unwrap();
    assert_eq!(c.retry_count, 0);

    // ... and update/query use their own optional semantics with renames kept.
    let u = GadgetUpdateRequest {
        display_name: Some("z".into()),
        retry_count: None,
    };
    assert_eq!(serde_json::to_string(&u).unwrap(), r#"{"label":"z"}"#);
    let d: GadgetDeleteRequest = serde_json::from_str(r#"{"gadgetId":9}"#).unwrap();
    assert_eq!(d.gadget_id, 9);
}

// --------------------------------------------------------------- serde_with

#[derive(Entity)]
pub struct Counter {
    #[entity(id)]
    pub id: Uuid,
    #[entity(serde_as = "DisplayFromStr")]
    pub total: u64,
    #[entity(serde_as = "DisplayFromStr")]
    pub limit: Option<u64>,
}

#[test]
fn serde_with_is_applied_with_option_wrapping() {
    let r = CounterResponse {
        id: Uuid::nil(),
        total: 7,
        limit: Some(9),
    };
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["total"], "7");
    assert_eq!(json["limit"], "9");
    let back: CounterResponse = serde_json::from_value(json).unwrap();
    assert_eq!((back.total, back.limit), (7, Some(9)));

    // Update wraps non-optional `total` in Option<DisplayFromStr>, omits None.
    let u = CounterUpdateRequest {
        total: Some(5),
        limit: None,
    };
    assert_eq!(serde_json::to_string(&u).unwrap(), r#"{"total":"5"}"#);
    let u: CounterUpdateRequest = serde_json::from_str(r#"{"limit":"12"}"#).unwrap();
    assert_eq!((u.total, u.limit), (None, Some(12)));

    let q: CounterQueryRequest = serde_json::from_str(r#"{"total":"3"}"#).unwrap();
    assert_eq!(q.total, Some(3));

    let c: CounterCreateRequest = serde_json::from_str(r#"{"total":"1"}"#).unwrap();
    assert_eq!((c.total, c.limit), (1, None));
}

// -------------------------------------------------------------- generics

#[derive(Entity)]
pub struct Wrapper<'a, T, U: Clone>
where
    T: Clone,
{
    #[entity(id)]
    pub id: u32,
    pub value: T,
    pub label: &'a str,
    #[entity(skip)]
    pub unused: Option<U>,
}

#[test]
fn generic_entities_keep_only_needed_params() {
    fn is_entity<E: EntityTrait>() {}
    is_entity::<Wrapper<'static, String, u8>>();

    // `U` is never used by a generated type, so none of them mention it.
    let r: WrapperResponse<'_, String> = WrapperResponse {
        id: 1,
        value: "v".to_string(),
        label: "l",
    };
    let _ = WrapperDeleteRequest { id: 1 }; // no generics at all here
    let u: WrapperUpdateRequest<'_, String> = WrapperUpdateRequest {
        value: None,
        label: Some("x"),
    };
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        r#"{"id":1,"value":"v","label":"l"}"#
    );
    assert_eq!(serde_json::to_string(&u).unwrap(), r#"{"label":"x"}"#);
}

/// Inherent-method probe: `is_entity()` is true only if `T: Entity` holds.
struct Probe<T>(std::marker::PhantomData<T>);
impl<T: EntityTrait> Probe<T> {
    fn is_entity(&self) -> bool {
        true
    }
}
trait Fallback {
    fn is_entity(&self) -> bool {
        false
    }
}
impl<T> Fallback for Probe<T> {}

#[test]
fn send_sync_bounds_follow_the_generic_parameters() {
    use std::marker::PhantomData;
    use std::rc::Rc;

    #[derive(Entity)]
    struct Holder<T> {
        #[entity(id)]
        #[allow(dead_code)]
        id: u32,
        #[allow(dead_code)]
        value: T,
    }
    #[derive(Entity)]
    struct Plain<T> {
        #[entity(id)]
        #[allow(dead_code)]
        id: u32,
        #[entity(skip)]
        #[allow(dead_code)]
        value: T,
    }

    assert!(Probe::<Holder<u8>>(PhantomData).is_entity());
    // The generated types hold `T`, so `T` must be Send + Sync ...
    assert!(!Probe::<Holder<Rc<u8>>>(PhantomData).is_entity());
    // ... but no bound is added for a parameter no generated type uses.
    assert!(Probe::<Plain<Rc<u8>>>(PhantomData).is_entity());
}

// ------------------------------------------------------------- validation

// `custom` receives `&String` here; deref coercion lets it take `&str`.
fn no_spaces(value: &str) -> Result<(), &'static str> {
    if value.contains(' ') {
        Err("must not contain spaces")
    } else {
        Ok(())
    }
}

#[derive(Entity)]
pub struct Tag {
    #[entity(id, validate(range(min = 1)))]
    pub id: u8,
    #[entity(validate(length(min = 1, max = 5), custom(no_spaces)))]
    pub name: String,
    #[entity(validate(length(max = 5)))]
    pub note: Option<String>,
    // `custom` always receives the declared type minus one `Option`.
    #[entity(validate(custom(no_spaces)))]
    pub alias: Option<String>,
    pub free: String,
}

/// `(field, message)` pairs of a failed validation.
fn failures(r: Result<(), Error>) -> Vec<(String, String)> {
    match r {
        Err(Error::Invalid(errors)) => errors.into_iter().map(|e| (e.field, e.message)).collect(),
        other => panic!("expected field errors, got {other:?}"),
    }
}

fn create(name: &str, note: Option<&str>) -> TagCreateRequest {
    TagCreateRequest {
        name: name.into(),
        note: note.map(Into::into),
        alias: None,
        free: String::new(),
    }
}

#[test]
fn create_runs_the_rules() {
    assert!(create("x", None).validate().is_ok());

    assert_eq!(
        failures(create("", None).validate()),
        [("name".to_string(), "length is lower than 1".to_string())]
    );

    // Optional field: validated only when present.
    assert_eq!(
        failures(create("x", Some("abcdefg")).validate()),
        [("note".to_string(), "length is greater than 5".to_string())]
    );
    assert!(create("x", Some("abc")).is_valid());
}

#[test]
fn all_failures_are_reported_together_and_structured() {
    let err = create("a b", Some("abcdefg")).validate().unwrap_err();
    let fields = err.fields();
    assert_eq!(fields.len(), 2);
    assert_eq!(
        (fields[0].field.as_str(), fields[0].message.as_str()),
        ("name", "must not contain spaces")
    );
    assert_eq!(fields[1].field, "note");
    assert_eq!(
        err.to_string(),
        "validation error: name: must not contain spaces; note: length is greater than 5"
    );
}

#[test]
fn update_requires_a_field_and_validates_values() {
    let empty = TagUpdateRequest {
        name: None,
        note: None,
        alias: None,
        free: None,
    };
    assert!(empty.is_empty());
    match empty.validate() {
        // Not tied to a single field, so it is a request-level error.
        Err(Error::ValidationError(m)) => assert!(m.contains("must set at least one field"), "{m}"),
        other => panic!("unexpected: {other:?}"),
    }
    assert!(empty.validate().unwrap_err().fields().is_empty());

    // Rules apply to the inner value of the `Option`-wrapped update field.
    let ok = TagUpdateRequest {
        name: Some("ok".into()),
        note: None,
        alias: None,
        free: None,
    };
    assert!(ok.is_valid());
    let bad = TagUpdateRequest {
        name: Some("".into()),
        note: None,
        alias: None,
        free: None,
    };
    assert_eq!(
        failures(bad.validate()),
        [("name".to_string(), "length is lower than 1".to_string())]
    );
    // A field without rules never fails.
    let free = TagUpdateRequest {
        name: None,
        note: None,
        alias: None,
        free: Some("anything".into()),
    };
    assert!(free.is_valid());
    // `custom` on a non-Option field sees the plain `String` in update requests too.
    let spaced = TagUpdateRequest {
        name: Some("a b".into()),
        note: None,
        alias: None,
        free: None,
    };
    assert_eq!(failures(spaced.validate())[0].1, "must not contain spaces");
    let alias = TagUpdateRequest {
        name: None,
        note: None,
        alias: Some("a b".into()),
        free: None,
    };
    assert_eq!(failures(alias.validate())[0].0, "alias");
}

#[test]
fn id_rules_apply_to_delete_and_query() {
    assert!(TagDeleteRequest { id: 1 }.is_valid());
    assert_eq!(
        failures(TagDeleteRequest { id: 0 }.validate()),
        [("id".to_string(), "lower than 1".to_string())]
    );
    assert!(!TagDeleteRequest { id: 1 }.is_empty());

    // An empty query means "match everything" and is valid.
    let q = TagQueryRequest {
        id: None,
        name: None,
        note: None,
        alias: None,
        free: None,
    };
    assert!(q.is_empty() && q.is_valid());
    let q = TagQueryRequest {
        id: Some(0),
        name: None,
        note: None,
        alias: None,
        free: None,
    };
    assert!(q.is_invalid());
}

#[derive(Entity)]
pub struct Rated<T> {
    #[entity(id)]
    pub id: u8,
    #[entity(validate(range(min = 1, max = 5)))]
    pub stars: u8,
    pub payload: T,
}

#[test]
fn rules_work_on_generic_entities() {
    let ok: RatedCreateRequest<String> = RatedCreateRequest {
        stars: 3,
        payload: "p".into(),
    };
    assert!(ok.is_valid());
    let bad: RatedCreateRequest<String> = RatedCreateRequest {
        stars: 9,
        payload: "p".into(),
    };
    assert_eq!(
        failures(bad.validate()),
        [("stars".to_string(), "greater than 5".to_string())]
    );
}
