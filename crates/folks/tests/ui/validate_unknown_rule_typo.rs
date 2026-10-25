use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(lenght(min = 1)))]
    name: String,
}

fn main() {}
