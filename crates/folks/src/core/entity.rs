use crate::core::error::Error;

pub trait Entity {
    type EntityId: Send + Sync;
    type EntityResponse: Send + Sync;
    type EntityCreateRequest: Send + Sync + Validatable;
    type EntityUpdateRequest: Send + Sync + Validatable;
    type EntityDeleteRequest: Send + Sync + Validatable;
    type EntityQueryRequest: Send + Sync + Validatable;
}

pub trait Validatable {
    fn is_empty(&self) -> bool;
    fn validate(&self) -> Result<(), Error>;
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
    fn is_invalid(&self) -> bool {
        !self.is_valid()
    }
}
