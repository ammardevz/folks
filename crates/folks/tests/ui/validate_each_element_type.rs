use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(each(length(max = 2))))]
    tags: Vec<u8>,
}

fn main() {}
