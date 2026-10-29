use folks::Entity;

struct Model {
    id: u8,
    disabled_at: Option<u64>,
}

struct Incompatible;

fn is_set(_value: Incompatible) -> bool {
    true
}

#[derive(Entity)]
#[entity(from(Model))]
struct User {
    #[entity(id)]
    id: u8,
    #[entity(map(from = "disabled_at", with = "is_set"))]
    disabled: bool,
}

fn main() {}
