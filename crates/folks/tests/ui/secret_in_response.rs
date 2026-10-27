use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(secret, response)]
    password: String,
}

fn main() {}
