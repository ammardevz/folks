use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    a: u32,
    #[entity(id)]
    b: u32,
}

fn main() {}
