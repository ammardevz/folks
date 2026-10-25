use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(update, default)]
    name: String,
}

fn main() {}
