use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(clearable, serde_as = "DisplayFromStr")]
    age: Option<u32>,
}

fn main() {}
