use folks::Entity;

#[derive(Entity)]
#[entity(serde(crate = "other"))]
struct User {
    #[entity(id)]
    id: u32,
}

fn main() {}
