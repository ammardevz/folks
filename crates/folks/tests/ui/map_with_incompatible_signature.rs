use folks::Entity;

struct Model {
    id: u8,
    disabled_at: Option<u64>,
}

fn is_set(value: String) -> bool {
    !value.is_empty()
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
