use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(skip, update)]
    name: String,
}

fn main() {}
