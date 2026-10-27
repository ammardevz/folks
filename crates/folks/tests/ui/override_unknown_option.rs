use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(create(name = "x"))]
    name: String,
}

fn main() {}
