use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(update, validate(required))]
    name: Option<String>,
}

fn main() {}
