//! Layered configuration (`shared` + per-type blocks), `strict`, `patch` mode,
//! the `Patch` trait and the prelude.

use folks::prelude::*;

// Shared settings apply everywhere; per-type blocks add to / override them.
#[derive(Entity)]
#[entity(
    rename_all = "camelCase",
    strict,
    derive(PartialEq),
    response(name = "UserDto", derive(Eq), strict = false),
    create(name = "NewUser"),
    update(name = "UserPatch")
)]
pub struct User {
    #[entity(id)]
    pub user_id: u32,
    pub full_name: String,
}

#[test]
fn shared_settings_reach_every_type_and_blocks_override() {
    // rename_all is shared.
    let r = UserDto {
        user_id: 1,
        full_name: "a".into(),
    };
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        r#"{"userId":1,"fullName":"a"}"#
    );

    // `strict` covers requests ...
    assert!(serde_json::from_str::<NewUser>(r#"{"fullName":"a","x":1}"#).is_err());
    assert!(serde_json::from_str::<UserPatch>(r#"{"x":1}"#).is_err());
    assert!(serde_json::from_str::<UserDeleteRequest>(r#"{"userId":1,"x":1}"#).is_err());
    // ... but not the response, and `strict = false` is explicit there anyway.
    assert!(serde_json::from_str::<UserDto>(r#"{"userId":1,"fullName":"a","x":1}"#).is_ok());

    // Shared derive(PartialEq) + response-only derive(Eq).
    fn eq<T: Eq>() {}
    eq::<UserDto>();
    assert_eq!(
        NewUser {
            full_name: "a".into()
        },
        NewUser {
            full_name: "a".into()
        }
    );
}

#[test]
fn response_can_opt_into_strict() {
    #[derive(Entity)]
    #[entity(response(strict))]
    struct S {
        #[entity(id)]
        #[allow(dead_code)]
        id: u8,
    }
    assert!(serde_json::from_str::<SResponse>(r#"{"id":1,"x":2}"#).is_err());
    // Requests were not made strict by the response block.
    assert!(serde_json::from_str::<SDeleteRequest>(r#"{"id":1,"x":2}"#).is_ok());
}

#[test]
fn config_can_be_split_over_several_attributes() {
    #[derive(Entity)]
    #[entity(name = "Item")]
    #[entity(derive(PartialEq))]
    #[entity(update(name = "ItemChange"))]
    struct Thing {
        #[entity(id)]
        #[allow(dead_code)]
        id: u8,
    }
    assert_eq!(ItemResponse { id: 1 }, ItemResponse { id: 1 });
    let _ = ItemChange::default();
}

// ----------------------------------------------------------- patch mode

#[derive(Entity, Debug, PartialEq)]
#[entity(patch)]
pub struct Contact {
    #[entity(id)]
    pub id: u8,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[entity(clearable = false)]
    pub nickname: Option<String>,
}

#[test]
fn patch_mode_makes_every_option_field_clearable_except_opt_outs() {
    // JSON merge patch: omitted = keep, null = clear, value = set.
    let u: ContactUpdateRequest =
        serde_json::from_str(r#"{"phone":null,"email":"e","nickname":null}"#).unwrap();
    assert_eq!(u.phone, Maybe::Null);
    assert_eq!(u.email, Maybe::Set("e".to_string()));
    assert_eq!(u.nickname, None); // opted out: null means "not supplied"
    assert_eq!(u.name, None);

    let mut c = Contact {
        id: 1,
        name: "n".into(),
        phone: Some("1".into()),
        email: None,
        nickname: Some("nick".into()),
    };
    u.apply_to(&mut c);
    assert_eq!(c.phone, None);
    assert_eq!(c.email.as_deref(), Some("e"));
    assert_eq!(c.nickname.as_deref(), Some("nick"));
    assert_eq!(c.name, "n");
}

// ------------------------------------------------- Patch trait, prelude

fn patch_it<E, P: Patch<E>>(patch: P, target: &mut E) {
    patch.apply_to(target);
}

#[test]
fn patch_trait_works_generically() {
    let mut c = Contact {
        id: 1,
        name: "n".into(),
        phone: None,
        email: None,
        nickname: None,
    };
    patch_it(ContactUpdateRequest::default().with_name("m"), &mut c);
    assert_eq!(c.name, "m");
}

#[test]
fn prelude_brings_the_basics() {
    let page = Page::new(vec![1, 2], 2, &PageRequest::default());
    assert_eq!(page.total_pages(), 1);
    assert!(ContactUpdateRequest::default().is_empty());
}
