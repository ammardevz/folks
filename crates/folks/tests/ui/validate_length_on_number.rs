use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(length(min = 1)))]
    age: u32,
}

fn main() {}
