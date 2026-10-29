use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u64,
    #[entity(validate(length(min = 1)))]
    score: Option<u32>,
}

fn main() {}
