use folks::Dto;

#[derive(Dto)]
#[entity(query(paginate))]
#[serde(rename_all = "camelCase")]
struct Project {
    #[entity(id)]
    id: u64,
    display_name: String,
}

#[test]
fn dto_is_an_entity_derive_alias() {
    let project = Project {
        id: 9,
        display_name: "library".into(),
    };
    let response: ProjectResponse = project.into();
    assert_eq!(
        (response.id, response.display_name.as_str()),
        (9, "library")
    );
    assert_eq!(
        serde_json::to_value(&response).unwrap()["displayName"],
        "library"
    );
    assert_eq!(ProjectQueryRequest::default().page_request().page, 1);
}
