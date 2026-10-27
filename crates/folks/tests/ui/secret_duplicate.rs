use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(secret, secret)]
    password: String,
}

fn main() {}
