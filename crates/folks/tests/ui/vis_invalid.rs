use folks::Entity;

#[derive(Entity)]
#[entity(vis = "not a visibility")]
struct User {
    #[entity(id)]
    id: u32,
}

fn main() {}
