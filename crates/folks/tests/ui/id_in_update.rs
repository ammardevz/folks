use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id, update)]
    id: u32,
}

fn main() {}
