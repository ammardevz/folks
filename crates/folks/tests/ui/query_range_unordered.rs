use folks::Entity;
#[derive(Entity)]
struct Item {
    #[entity(id)]
    id: u64,
    #[entity(query(range))]
    active: bool,
}
fn main() {}
