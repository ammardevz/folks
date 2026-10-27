use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id, create(ty = String))]
    id: u32,
}

fn main() {}
