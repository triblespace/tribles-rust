use ed25519::{ComponentBytes, Signature};
use ed25519_dalek::SignatureError;
pub use ed25519_dalek::VerifyingKey;
use hex_literal::hex;

use crate::id::RawId;
use crate::value::{FromValue, ToValue, TryFromValue, Value, ValueSchema};

pub struct ED25519RComponent;
pub struct ED25519SComponent;
pub struct ED25519PublicKey;

impl ValueSchema for ED25519RComponent {
    const ID: RawId = hex!("995A86FFC83DB95ECEAA17E226208897");
}
impl ValueSchema for ED25519SComponent {
    const ID: RawId = hex!("10D35B0B628E9E409C549D8EC1FB3598");
}
impl ValueSchema for ED25519PublicKey {
    const ID: RawId = hex!("69A872254E01B4C1ED36E08E40445E93");
}

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

impl ToValue<ED25519RComponent> for ComponentBytes {
    fn to_value(self) -> Value<ED25519RComponent> {
        Value::new(self)
    }
}

impl FromValue<'_, ED25519RComponent> for ComponentBytes {
    fn from_value(v: &Value<ED25519RComponent>) -> Self {
        v.bytes
    }
}

impl ToValue<ED25519SComponent> for ComponentBytes {
    fn to_value(self) -> Value<ED25519SComponent> {
        Value::new(self)
    }
}

impl FromValue<'_, ED25519SComponent> for ComponentBytes {
    fn from_value(v: &Value<ED25519SComponent>) -> Self {
        v.bytes
    }
}

impl ToValue<ED25519PublicKey> for VerifyingKey {
    fn to_value(self) -> Value<ED25519PublicKey> {
        Value::new(self.to_bytes())
    }
}

impl TryFromValue<'_, ED25519PublicKey> for VerifyingKey {
    type Error = SignatureError;

    fn try_from_value(v: &Value<ED25519PublicKey>) -> Result<Self, Self::Error> {
        VerifyingKey::from_bytes(&v.bytes)
    }
}
