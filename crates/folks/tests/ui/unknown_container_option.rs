use folks::Entity;

#[derive(Entity)]
#[entity(skip)]
struct User {
    #[entity(id)]
    id: u32,
}

fn main() {}
