use ed25519::{ComponentBytes, Signature};
pub use ed25519_dalek::VerifyingKey;

use crate::types::{ValueParseError, Valuelike};

#[derive(Debug)]
pub struct RComponent(pub ComponentBytes);
#[derive(Debug)]
pub struct SComponent(pub ComponentBytes);

impl RComponent {
    pub fn from_signature(s: Signature) -> Self {
        Self(*s.r_bytes())
    }
}

impl SComponent {
    pub fn from_signature(s: Signature) -> Self {
        Self(*s.s_bytes())
    }
}

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
        VerifyingKey::from_bytes(&value)
            .map_err(|_| ValueParseError::new(value, "failed to construct valid VerifyingKey"))
    }

    fn into_value(&self) -> crate::types::Value {
        self.to_bytes()
    }
}
