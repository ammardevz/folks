use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u8,
    #[entity(map(from = "email_address"))]
    email: String,
}

fn main() {}
