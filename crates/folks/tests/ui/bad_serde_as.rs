use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(serde_as = "not a type !!")]
    name: String,
}

fn main() {}
