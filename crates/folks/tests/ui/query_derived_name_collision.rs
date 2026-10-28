use folks::Entity;
#[derive(Entity)]
struct Item {
    #[entity(id)]
    id: u64,
    #[entity(query(range))]
    created: i64,
    created_from: String,
}
fn main() {}
