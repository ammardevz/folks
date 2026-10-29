use folks::Entity;

struct Model {
    id: u8,
    name: String,
}

#[derive(Entity)]
#[entity(from(Model))]
struct User {
    #[entity(id)]
    id: u8,
    #[entity(create, map(from = "name"))]
    name: String,
}

fn main() {}
