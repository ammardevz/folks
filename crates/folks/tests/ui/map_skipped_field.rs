use folks::Entity;

struct Model {
    id: u8,
}

#[derive(Entity)]
#[entity(from(Model))]
struct User {
    #[entity(id)]
    id: u8,
    #[entity(skip, map(from = "internal_name"))]
    name: String,
}

fn main() {}
