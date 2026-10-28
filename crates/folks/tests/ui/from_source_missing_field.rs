use folks::Entity;

struct Model {
    id: u64,
}

#[derive(Entity)]
#[entity(from(Model))]
struct User {
    #[entity(id)]
    id: u64,
    name: String,
}

fn main() {}
