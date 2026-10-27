use folks::Entity;

#[derive(Entity)]
#[entity(matches)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(query(ty = u8))]
    age: u32,
}

fn main() {}
