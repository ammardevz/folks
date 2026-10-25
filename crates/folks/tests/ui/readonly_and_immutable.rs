use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(readonly, immutable)]
    name: String,
}

fn main() {}
