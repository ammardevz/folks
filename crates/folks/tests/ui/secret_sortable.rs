use folks::Entity;
#[derive(Entity)]
struct Item {
    #[entity(id)]
    id: u64,
    #[entity(secret, sortable)]
    key: String,
}
fn main() {}
