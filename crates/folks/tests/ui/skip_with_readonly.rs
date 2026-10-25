use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(skip, readonly)]
    name: String,
}

fn main() {}
