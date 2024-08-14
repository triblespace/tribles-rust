use std::convert::TryFrom;

use ed25519::{ComponentBytes, Signature};
use ed25519_dalek::SignatureError;
pub use ed25519_dalek::VerifyingKey;

use crate::{ Value, Schema};

pub struct ED25519RComponent;
pub struct ED25519SComponent;
pub struct ED25519PublicKey;

impl Schema for ED25519RComponent {}
impl Schema for ED25519SComponent {}
impl Schema for ED25519PublicKey {}

impl ED25519RComponent {
    pub fn from_signature(s: Signature) -> Value<ED25519RComponent> {
        Value::new(*s.r_bytes())
    }
}

impl ED25519SComponent {
    pub fn from_signature(s: Signature) -> Value<ED25519SComponent> {
        Value::new(*s.s_bytes())
    }
}

impl From<ComponentBytes> for Value<ED25519RComponent> {
    fn from(value: ComponentBytes) -> Self {
        Value::new(value)
    }
}

impl From<Value<ED25519RComponent>> for ComponentBytes {
    fn from(value: Value<ED25519RComponent>) -> Self {
        value.bytes
    }
}

impl From<ComponentBytes> for Value<ED25519SComponent> {
    fn from(value: ComponentBytes) -> Self {
        Value::new(value)
    }
}

impl From<Value<ED25519SComponent>> for ComponentBytes {
    fn from(value: Value<ED25519SComponent>) -> Self {
        value.bytes
    }
}

impl TryFrom<Value<ED25519PublicKey>> for VerifyingKey {    
    type Error = SignatureError;
    
    fn try_from(value: Value<ED25519PublicKey>) -> Result<Self, Self::Error> {
        VerifyingKey::from_bytes(&value.bytes)
    }
}

impl From<VerifyingKey> for Value<ED25519PublicKey> {
    fn from(value: VerifyingKey) -> Self {
        Value::new(value.to_bytes())
    }
}
