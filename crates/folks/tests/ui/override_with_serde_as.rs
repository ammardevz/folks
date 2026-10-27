use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(create(ty = String), serde_as = "folks::serde_with::DisplayFromStr")]
    age: u32,
}

fn main() {}
