use garde::Validate;

/// Ties an entity to the request and response types generated for it.
///
/// The request types implement [`garde::Validate`] with no context, so they
/// work with every garde integration (`axum-valid`, `actix-web-garde`, ...).
pub trait Entity {
    type EntityId: Send + Sync;
    type EntityResponse: Send + Sync;
    type EntityCreateRequest: Send + Sync + Validate<Context = ()>;
    type EntityUpdateRequest: Send + Sync + Validate<Context = ()>;
    type EntityDeleteRequest: Send + Sync + Validate<Context = ()>;
    type EntityQueryRequest: Send + Sync + Validate<Context = ()>;
}
