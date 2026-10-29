use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u8,
    #[entity(create(ty = u32, validate(length(min = 1))))]
    name: String,
}

fn main() {}
