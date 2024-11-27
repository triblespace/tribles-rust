use crate::blob::BlobSchema;
use crate::id::RawId;
use crate::value::{FromValue, RawValue, TryToValue, Value, ValueSchema};

use anybytes::Bytes;
use digest::{typenum::U32, Digest};
use hex::{FromHex, FromHexError};
use hex_literal::hex;
use std::marker::PhantomData;

pub trait HashProtocol: Digest<OutputSize = U32> + 'static {
    const NAME: &'static str;
    const SCHEMA_ID: RawId;
}

pub struct Hash<H> {
    _hasher: PhantomData<H>,
}

impl<H> ValueSchema for Hash<H>
where
    H: HashProtocol,
{
    const ID: RawId = H::SCHEMA_ID;
}

impl<H> Hash<H>
where
    H: HashProtocol,
{
    pub fn digest(blob: &Bytes) -> Value<Self> {
        Value::new(H::digest(&blob).into())
    }

    pub fn from_hex(hex: &str) -> Result<Value<Self>, FromHexError> {
        let digest = RawValue::from_hex(hex)?;
        Ok(Value::new(digest))
    }

    pub fn to_hex(value: &Value<Self>) -> String {
        hex::encode_upper(value.raw)
    }
}

impl<H> FromValue<'_, Hash<H>> for String
where
    H: HashProtocol,
{
    fn from_value(v: &Value<Hash<H>>) -> Self {
        let mut out = String::new();
        out.push_str(<H as HashProtocol>::NAME);
        out.push(':');
        out.push_str(&hex::encode(v.raw));
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HashError {
    BadProtocol,
    BadHex(FromHexError),
}

impl From<FromHexError> for HashError {
    fn from(value: FromHexError) -> Self {
        HashError::BadHex(value)
    }
}

impl<H> TryToValue<Hash<H>> for &str
where
    H: HashProtocol,
{
    type Error = HashError;

    fn try_to_value(self) -> Result<Value<Hash<H>>, Self::Error> {
        let protocol = <H as HashProtocol>::NAME;
        if !(self.starts_with(protocol) && &self[protocol.len()..=protocol.len()] == ":") {
            return Err(HashError::BadProtocol);
        }
        let digest = RawValue::from_hex(&self[protocol.len() + 1..])?;

        Ok(Value::new(digest))
    }
}

impl<H> TryToValue<Hash<H>> for String
where
    H: HashProtocol,
{
    type Error = HashError;

    fn try_to_value(self) -> Result<Value<Hash<H>>, Self::Error> {
        (&self[..]).try_to_value()
    }
}

impl<H: HashProtocol, T: BlobSchema> From<Value<Hash<H>>> for Value<Handle<H, T>> {
    fn from(value: Value<Hash<H>>) -> Self {
        Value::new(value.raw)
    }
}

use blake2::Blake2b as Blake2bUnsized;
pub type Blake2b = Blake2bUnsized<U32>;

pub use blake3::Hasher as Blake3;

impl HashProtocol for Blake2b {
    const NAME: &'static str = "blake2";
    const SCHEMA_ID: RawId = hex!("91F880222412A49F012BE999942E6199");
}

impl HashProtocol for Blake3 {
    const NAME: &'static str = "blake3";
    const SCHEMA_ID: RawId = hex!("4160218D6C8F620652ECFBD7FDC7BDB3");
}

#[repr(transparent)]
pub struct Handle<H, T: BlobSchema> {
    digest: Hash<H>,
    _type: PhantomData<T>,
}

impl<H: HashProtocol, T: BlobSchema> From<Value<Handle<H, T>>> for Value<Hash<H>> {
    fn from(value: Value<Handle<H, T>>) -> Self {
        Value::new(value.raw)
    }
}

impl<H: HashProtocol, T: BlobSchema> ValueSchema for Handle<H, T> {
    const ID: RawId = H::SCHEMA_ID;
}

#[cfg(test)]
mod tests {
    use super::Blake3;
    use crate::prelude::*;
    use crate::value::schemas::hash::HashError;
    use rand;

    use super::Hash;

    #[test]
    fn value_roundtrip() {
        let v: Value<Hash<Blake3>> = Value::new(rand::random());
        let s: String = v.from_value();
        let _: Value<Hash<Blake3>> = s.try_to_value().expect("roundtrip should succeed");
    }

    #[test]
    fn value_from_known() {
        let s: &str = "blake3:CA98593CB9DC0FA48B2BE01E53D042E22B47862D646F9F19E2889A7961663663";
        let _: Value<Hash<Blake3>> = s
            .try_to_value()
            .expect("packing valid constant should succeed");
    }

    #[test]
    fn to_value_fail_protocol() {
        let s: &str = "bad:CA98593CB9DC0FA48B2BE01E53D042E22B47862D646F9F19E2889A7961663663";
        let err: HashError = <&str as TryToValue<Hash<Blake3>>>::try_to_value(s)
            .expect_err("packing invalid protocol should fail");
        assert_eq!(err, HashError::BadProtocol);
    }

    #[test]
    fn to_value_fail_hex() {
        let s: &str = "blake3:BAD!";
        let err: HashError = <&str as TryToValue<Hash<Blake3>>>::try_to_value(s)
            .expect_err("packing invalid protocol should fail");
        assert!(matches!(err, HashError::BadHex(..)));
    }
}
