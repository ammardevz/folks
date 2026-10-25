use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id, create)]
    id: u32,
}

fn main() {}
