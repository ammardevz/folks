use folks::Entity;

#[derive(Entity)]
#[entity(response(name = "UserCreateRequest"))]
struct User {
    #[entity(id)]
    id: u32,
}

fn main() {}
