use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(update, update)]
    name: String,
}

fn main() {}
