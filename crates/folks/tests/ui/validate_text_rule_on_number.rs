use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(ascii))]
    age: u8,
}

fn main() {}
