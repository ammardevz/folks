use folks::Entity;
#[derive(Entity)]
struct Item {
    #[entity(id)]
    id: u64,
    #[entity(query(contains))]
    score: u32,
}
fn main() {}
