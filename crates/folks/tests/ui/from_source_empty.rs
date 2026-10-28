use folks::Entity;
#[derive(Entity)]
#[entity(from())]
struct User {
    #[entity(id)]
    id: u64,
}
fn main() {}
