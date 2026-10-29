use folks::Entity;

struct Model {
    id: u8,
    first_name: String,
    second_name: String,
}

#[derive(Entity)]
#[entity(from(Model))]
struct User {
    #[entity(id)]
    id: u8,
    #[entity(map(from = "first_name", from = "second_name"))]
    name: String,
}

fn main() {}
