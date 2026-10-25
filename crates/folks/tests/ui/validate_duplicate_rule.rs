use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(length(min = 1), length(max = 5)))]
    name: String,
}

fn main() {}
