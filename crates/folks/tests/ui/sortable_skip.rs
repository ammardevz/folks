use folks::Entity;
#[derive(Entity)]
struct Item {
    #[entity(id)]
    id: u64,
    #[entity(skip, sortable)]
    name: String,
}
fn main() {}
