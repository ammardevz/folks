//! Generated request types work with `axum-valid`'s `Garde<Json<T>>`.

use axum::body::Body;
use axum::extract::{FromRequest, Json};
use axum::http::{Request, header};
use axum_valid::Garde;
use folks_consumer::CustomerCreateRequest;

fn json_request(body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

type Extracted = Garde<Json<CustomerCreateRequest>>;

#[tokio::test]
async fn accepts_valid_input() {
    let req = json_request(r#"{"name":"Ann","tags":["a"],"age":30}"#);
    let Garde(Json(create)) = Extracted::from_request(req, &()).await.ok().unwrap();
    assert_eq!(create.name, "Ann");
}

#[tokio::test]
async fn rejects_invalid_input() {
    let req = json_request(r#"{"name":"","tags":[""],"age":200}"#);
    assert!(Extracted::from_request(req, &()).await.is_err());
}

#[tokio::test]
async fn rejects_malformed_input() {
    let req = json_request(r#"{"name":"Ann"}"#);
    assert!(Extracted::from_request(req, &()).await.is_err());
}
