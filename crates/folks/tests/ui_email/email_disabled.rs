use folks::Entity;

// Compiled only without the `email` feature.
#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(email))]
    mail: String,
}

fn main() {}
