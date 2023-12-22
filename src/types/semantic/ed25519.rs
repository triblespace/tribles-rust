use ed25519::ComponentBytes;
pub use ed25519_dalek::VerifyingKey;

use crate::types::{Valuelike, ValueParseError};

pub struct RComponent(ComponentBytes);
pub struct SComponent(ComponentBytes);

impl Valuelike for RComponent {
    fn from_value(value: crate::types::Value) -> Result<Self, ValueParseError> {
        Ok(RComponent(value))
    }

    fn into_value(&self) -> crate::types::Value {
        self.0
    }
}

impl Valuelike for SComponent {
    fn from_value(value: crate::types::Value) -> Result<Self, ValueParseError> {
        Ok(SComponent(value))
    }

    fn into_value(&self) -> crate::types::Value {
        self.0
    }
}

impl Valuelike for VerifyingKey {
    fn from_value(value: crate::types::Value) -> Result<Self, ValueParseError> {
        VerifyingKey::from_bytes(&value).map_err(|e| ValueParseError::new(value, "failed to construct valid VerifyingKey"))
    }

    fn into_value(&self) -> crate::types::Value {
        todo!()
    }
}
