use folks::Entity;
#[derive(Entity)]
#[entity(response(name = "ItemSortField"))]
struct Item {
    #[entity(id)]
    id: u64,
    #[entity(sortable)]
    name: String,
}
fn main() {}
