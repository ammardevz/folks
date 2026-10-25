use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(frobnicate)]
    name: String,
}

fn main() {}
