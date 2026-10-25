use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id, skip)]
    id: u32,
}

fn main() {}
