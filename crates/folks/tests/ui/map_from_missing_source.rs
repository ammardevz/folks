use folks::Entity;

struct Model {
    id: u8,
}

fn is_set<T>(value: Option<T>) -> bool {
    value.is_some()
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
