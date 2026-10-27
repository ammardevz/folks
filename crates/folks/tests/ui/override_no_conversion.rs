use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    #[entity(response(ty = u8), create)]
    score: f64,
}

fn main() {}
