use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u8,
    #[entity(normalize(lowercase, snake_case))]
    name: String,
}

fn main() {}
