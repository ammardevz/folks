use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u8,
    #[entity(normalize(trim))]
    age: u32,
}

fn main() {}
