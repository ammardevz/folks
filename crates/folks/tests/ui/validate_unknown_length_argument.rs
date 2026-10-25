use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(length(minimum = 1)))]
    name: String,
}

fn main() {}
