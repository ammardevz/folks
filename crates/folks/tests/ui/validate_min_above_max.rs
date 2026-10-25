use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(length(min = 5, max = 1)))]
    name: String,
}

fn main() {}
