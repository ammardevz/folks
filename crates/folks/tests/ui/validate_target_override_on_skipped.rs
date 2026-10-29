use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(skip, create(validate(length(min = 1))))]
    name: String,
}

fn main() {}
