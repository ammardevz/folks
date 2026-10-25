use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id, readonly)]
    id: u32,
}

fn main() {}
