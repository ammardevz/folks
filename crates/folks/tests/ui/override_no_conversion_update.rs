use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(response, update(ty = f64))]
    age: u8,
}

fn main() {}
