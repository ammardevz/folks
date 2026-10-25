use folks::Entity;

#[derive(Entity)]
#[entity(response(colour = "red"))]
struct User {
    #[entity(id)]
    id: u32,
}

fn main() {}
