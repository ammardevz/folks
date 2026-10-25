use folks::Entity;

#[derive(Entity)]
#[entity(no(everything))]
struct User {
    #[entity(id)]
    id: u32,
}

fn main() {}
