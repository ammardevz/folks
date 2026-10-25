use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(contains(5)))]
    name: String,
}

fn main() {}
