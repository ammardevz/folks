use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(response, validate(length(min = 1)))]
    name: String,
}

fn main() {}
