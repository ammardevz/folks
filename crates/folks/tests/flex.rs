//! Container knobs (`names`, `no`, `vis`, `serde`), create `default`, `clearable`.

use folks::{Entity, Maybe, Patch, Validatable};

// ------------------------------------------------------------------ names

#[derive(Entity)]
#[entity(
    response(name = "UserDto"),
    create(name = "NewUser"),
    update(name = "UserPatch"),
    delete(name = "UserRemoval"),
    query(name = "UserFilter")
)]
pub struct User {
    #[entity(id)]
    pub id: u32,
    pub name: String,
}

#[test]
fn names_override_every_type() {
    fn same<A, B>()
    where
        A: 'static,
        B: 'static,
    {
        assert_eq!(std::any::TypeId::of::<A>(), std::any::TypeId::of::<B>());
    }
    same::<<User as folks::Entity>::EntityResponse, UserDto>();
    same::<<User as folks::Entity>::EntityCreateRequest, NewUser>();
    same::<<User as folks::Entity>::EntityUpdateRequest, UserPatch>();
    same::<<User as folks::Entity>::EntityDeleteRequest, UserRemoval>();
    same::<<User as folks::Entity>::EntityQueryRequest, UserFilter>();
}

// ------------------------------------------------------------- no(..)

#[derive(Entity)]
#[entity(no(from, setters, patch, default))]
pub struct Manual {
    #[entity(id)]
    pub id: u32,
    pub title: String,
}

// With the helpers off, hand-written versions do not collide.
impl From<Manual> for ManualResponse {
    fn from(m: Manual) -> Self {
        ManualResponse {
            id: m.id + 1000,
            title: m.title,
        }
    }
}
impl Default for ManualUpdateRequest {
    fn default() -> Self {
        ManualUpdateRequest {
            title: Some("preset".into()),
        }
    }
}
impl ManualUpdateRequest {
    pub fn with_title(self, _: u8) -> Self {
        self
    }
    pub fn apply_to(self, target: &mut Manual) {
        target.title = "custom".into();
    }
}

#[test]
fn helpers_can_be_turned_off() {
    let r: ManualResponse = Manual {
        id: 1,
        title: "t".into(),
    }
    .into();
    assert_eq!(r.id, 1001);
    let mut m = Manual {
        id: 1,
        title: "t".into(),
    };
    ManualUpdateRequest::default()
        .with_title(0)
        .apply_to(&mut m);
    assert_eq!(m.title, "custom");
}

// ------------------------------------------------------------------- vis

mod hidden {
    use folks::Entity;

    // A private entity can still expose public DTOs.
    #[derive(Entity)]
    #[entity(vis = "pub")]
    #[allow(dead_code)]
    struct Internal {
        #[entity(id)]
        id: u8,
        secret: String,
    }
}

#[test]
fn vis_can_widen_generated_types() {
    let _ = hidden::InternalResponse {
        id: 1,
        secret: "s".into(),
    };
    let _ = hidden::InternalCreateRequest { secret: "s".into() };
}

// ----------------------------------------------------------------- serde

#[derive(Entity)]
#[entity(serde(deny_unknown_fields, rename_all = "camelCase"))]
pub struct Strict {
    #[entity(id)]
    pub strict_id: u8,
    pub display_name: String,
}

#[test]
fn extra_serde_attributes_reach_every_type() {
    let ok: StrictCreateRequest = serde_json::from_str(r#"{"displayName":"a"}"#).unwrap();
    assert_eq!(ok.display_name, "a");
    assert!(serde_json::from_str::<StrictCreateRequest>(r#"{"displayName":"a","x":1}"#).is_err());
    assert!(serde_json::from_str::<StrictUpdateRequest>(r#"{"nope":1}"#).is_err());
    let r = StrictResponse {
        strict_id: 1,
        display_name: "d".into(),
    };
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        r#"{"strictId":1,"displayName":"d"}"#
    );
}

// -------------------------------------------------------- create defaults

fn default_role() -> String {
    "member".to_string()
}

#[derive(Entity)]
pub struct Signup {
    #[entity(id)]
    pub id: u8,
    pub email: String,
    #[entity(default)]
    pub retries: u32,
    #[entity(default = "default_role")]
    pub role: String,
}

#[test]
fn create_defaults_fill_omitted_fields() {
    let c: SignupCreateRequest = serde_json::from_str(r#"{"email":"a@b"}"#).unwrap();
    assert_eq!((c.retries, c.role.as_str()), (0, "member"));
    let c: SignupCreateRequest =
        serde_json::from_str(r#"{"email":"a@b","retries":3,"role":"admin"}"#).unwrap();
    assert_eq!((c.retries, c.role.as_str()), (3, "admin"));
    // Required fields stay required, and responses are not affected.
    assert!(serde_json::from_str::<SignupCreateRequest>("{}").is_err());
    assert!(serde_json::from_str::<SignupResponse>(r#"{"id":1,"email":"a"}"#).is_err());
}

// ------------------------------------------------------------- clearable

#[derive(Entity, Debug, PartialEq)]
pub struct Profile {
    #[entity(id)]
    pub id: u8,
    #[entity(clearable)]
    pub bio: Option<String>,
    pub nick: Option<String>,
}

#[test]
fn clearable_distinguishes_omitted_from_null() {
    let omitted: ProfileUpdateRequest = serde_json::from_str(r#"{"nick":"n"}"#).unwrap();
    assert_eq!(omitted.bio, Maybe::Unset);
    let null: ProfileUpdateRequest = serde_json::from_str(r#"{"bio":null}"#).unwrap();
    assert_eq!(null.bio, Maybe::Null);
    let set: ProfileUpdateRequest = serde_json::from_str(r#"{"bio":"hi"}"#).unwrap();
    assert_eq!(set.bio, Maybe::Set("hi".to_string()));

    // Serialization round-trips the three states.
    assert_eq!(serde_json::to_string(&null).unwrap(), r#"{"bio":null}"#);
    assert_eq!(serde_json::to_string(&set).unwrap(), r#"{"bio":"hi"}"#);
    assert_eq!(serde_json::to_string(&omitted).unwrap(), r#"{"nick":"n"}"#);

    // A non-clearable `Option<T>` field stays flat: `null` means "not supplied".
    let flat: ProfileUpdateRequest = serde_json::from_str(r#"{"nick":null}"#).unwrap();
    assert_eq!(flat.nick, None);
}

#[test]
fn clearable_setters_and_apply_to() {
    let mut p = Profile {
        id: 1,
        bio: Some("old".into()),
        nick: Some("n".into()),
    };
    ProfileUpdateRequest::default()
        .with_bio("new")
        .apply_to(&mut p);
    assert_eq!(p.bio.as_deref(), Some("new"));

    ProfileUpdateRequest::default().clear_bio().apply_to(&mut p);
    assert_eq!(p.bio, None);
    assert_eq!(p.nick.as_deref(), Some("n")); // untouched

    // Clearing counts as a supplied field.
    let clear = ProfileUpdateRequest::default().clear_bio();
    assert!(!clear.is_empty());
    assert!(clear.is_valid());
}

mod clearable_validation {
    use super::*;

    #[derive(Entity)]
    pub struct Bio {
        #[entity(id)]
        pub id: u8,
        #[entity(clearable, validate(length(max = 5)))]
        pub text: Option<String>,
    }

    #[test]
    fn clearable_fields_keep_their_validation() {
        assert!(
            BioUpdateRequest::default()
                .with_text("abcdefg")
                .is_invalid()
        );
        assert!(BioUpdateRequest::default().with_text("abc").is_valid());
        // Null is a valid (clearing) update; unset alone is an empty one.
        assert!(BioUpdateRequest::default().clear_text().is_valid());
        assert!(BioUpdateRequest::default().is_invalid());
    }
}
