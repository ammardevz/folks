use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id, delete)]
    id: u32,
}

fn main() {}
