use ed25519::{ComponentBytes, Signature};
use ed25519_dalek::SignatureError;
pub use ed25519_dalek::VerifyingKey;

use crate::{ValueSchema, Value};

use super::{PackValue, TryUnpackValue, UnpackValue};

pub struct ED25519RComponent;
pub struct ED25519SComponent;
pub struct ED25519PublicKey;

impl ValueSchema for ED25519RComponent {}
impl ValueSchema for ED25519SComponent {}
impl ValueSchema for ED25519PublicKey {}

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

impl PackValue<ED25519RComponent> for ComponentBytes {
    fn pack(&self) -> Value<ED25519RComponent> {
        Value::new(*self)
    }
}

impl UnpackValue<'_, ED25519RComponent> for ComponentBytes {
    fn unpack(v: &Value<ED25519RComponent>) -> Self {
        v.bytes
    }
}

impl PackValue<ED25519SComponent> for ComponentBytes {
    fn pack(&self) -> Value<ED25519SComponent> {
        Value::new(*self)
    }
}

impl UnpackValue<'_, ED25519SComponent> for ComponentBytes {
    fn unpack(v: &Value<ED25519SComponent>) -> Self {
        v.bytes
    }
}

impl PackValue<ED25519PublicKey> for VerifyingKey {
    fn pack(&self) -> Value<ED25519PublicKey> {
        Value::new(self.to_bytes())
    }
}

impl TryUnpackValue<'_, ED25519PublicKey> for VerifyingKey {
    type Error = SignatureError;

    fn try_unpack(v: &Value<ED25519PublicKey>) -> Result<Self, Self::Error> {
        VerifyingKey::from_bytes(&v.bytes)
    }
}
