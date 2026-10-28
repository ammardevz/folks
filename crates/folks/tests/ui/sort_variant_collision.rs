use folks::Entity;
#[derive(Entity)]
struct Item {
    #[entity(id)]
    id: u64,
    #[entity(sortable)]
    created_at: i64,
    #[entity(sortable)]
    createdAt: i64,
}
fn main() {}
