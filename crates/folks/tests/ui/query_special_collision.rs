use folks::Entity;
#[derive(Entity)]
#[entity(query(paginate))]
struct Item {
    #[entity(id)]
    id: u64,
    page: u32,
}
fn main() {}
