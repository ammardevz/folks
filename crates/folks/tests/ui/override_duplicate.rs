use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(create(ty = u8, ty = u16))]
    age: u32,
}

fn main() {}
