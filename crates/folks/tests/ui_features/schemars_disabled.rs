// Compiled only when the `schemars` feature is off.
use folks::Entity;

#[derive(Entity)]
#[entity(schemars)]
struct User {
    #[entity(id)]
    id: u32,
}

fn main() {}
