use folks::Entity;

#[derive(Entity)]
struct User {
    #[entity(id)]
    id: u32,
    // `length` is fine for the entity's `String`, not for the `u32` in create requests.
    #[entity(create(ty = u32), update, validate(length(min = 1)))]
    name: String,
}

fn main() {}
