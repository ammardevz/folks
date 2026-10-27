use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(update(ty = Option<String>), clearable)]
    bio: Option<String>,
}

fn main() {}
