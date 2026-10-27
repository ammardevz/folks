use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(secret, skip)]
    password: String,
}

fn main() {}
