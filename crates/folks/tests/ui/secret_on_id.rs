use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id, secret)]
    id: u32,
}

fn main() {}
