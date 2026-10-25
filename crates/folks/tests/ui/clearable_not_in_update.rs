use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(clearable, create)]
    name: Option<String>,
}

fn main() {}
