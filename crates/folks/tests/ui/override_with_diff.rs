use folks::Entity;

#[derive(Entity)]
#[entity(diff)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(update(ty = u8))]
    age: u32,
}

fn main() {}
