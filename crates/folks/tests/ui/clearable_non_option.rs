use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(clearable)]
    name: String,
}

fn main() {}
