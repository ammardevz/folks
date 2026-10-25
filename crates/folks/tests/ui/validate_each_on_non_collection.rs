use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(validate(each(length(max = 2))))]
    name: String,
}

fn main() {}
