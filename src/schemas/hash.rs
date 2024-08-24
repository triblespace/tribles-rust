use std::marker::PhantomData;
use digest::{Digest, typenum::U32};
use anybytes::Bytes;
use crate::{ Handle, RawValue, Schema, Value };
use hex::{FromHex, FromHexError};

trait HashProtocol: Digest<OutputSize = U32> {
    const NAME: &'static str;
}

pub struct Hash<H> {
    _hasher: PhantomData<H>,
}

impl<H> Schema for Hash<H> {}

impl<H> Hash<H>
where
    H: Digest<OutputSize = U32>,
{
    pub fn digest(blob: &Bytes) -> Value<Self> {
        Value::new(H::digest(&blob).into())
    }

    pub fn from_hex(hex: &str) -> Result<Value<Self>, FromHexError> {

        let digest = RawValue::from_hex(hex)?;
        Ok(Value::new(digest))
    }

    pub fn to_hex(value: &Value::<Self>) -> String {
        hex::encode_upper(value.bytes)   
    }
}

impl<H> Unpack<'_, Hash<H>> for String
where H: HashProtocol {
    fn unpack(v: &Value<Hash<H>>) -> Self {
        let mut out = String::new();
        out.push_str(<H as HashProtocol>::NAME);
        out.push(':');
        out.push_str(&hex::encode(v.bytes));
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackHashError {
    BadProtocol,
    BadHex(FromHexError)
}

impl From<FromHexError> for PackHashError {
    fn from(value: FromHexError) -> Self {
        PackHashError::BadHex(value)
    }
}

impl<H> TryPack<Hash<H>> for str
where H: HashProtocol {
    type Error = PackHashError;
    
    fn try_pack(&self) -> Result<Value<Hash<H>>, Self::Error> {
        let method = <H as HashProtocol>::NAME;
        if !(self.starts_with(method) &&
             &self[method.len()..=method.len()] == ":"){
            return Err(PackHashError::BadProtocol)
        }
       let digest = RawValue::from_hex(&self[method.len() + 1..])?;
        
        Ok(Value::new(digest))
    }
}

impl<H, T> From<Value<Hash<H>>> for Value<Handle<H, T>> {
    fn from(value: Value<Hash<H>>) -> Self {
        Value::new(value.bytes)
    }
}


use blake2::Blake2b as Blake2bUnsized;
pub type Blake2b = Blake2bUnsized<U32>;

pub use blake3::Hasher as Blake3;

impl HashProtocol for Blake2b {
    const NAME: &'static str = "blake2";
}

impl HashProtocol for Blake3 {
    const NAME: &'static str = "blake3";
}

use super::{TryPack, Unpack};

#[cfg(test)]
mod tests {
    use super::Blake3;
    use crate::{schemas::{hash::PackHashError, TryPack}, Value};
    use rand;

    use super::Hash;

    #[test]
    fn unpack_pack() {
        let v: Value<Hash<Blake3>> = Value::new(rand::random());
        let s: String = v.unpack();
        let _: Value<Hash<Blake3>>  = s.try_pack().expect("roundtrip should succeed");
    }

    #[test]
    fn pack_known() {
        let s: &str = "blake3:CA98593CB9DC0FA48B2BE01E53D042E22B47862D646F9F19E2889A7961663663";
        let _: Value<Hash<Blake3>>  = s.try_pack().expect("packing valid constant should succeed");
    }

    #[test]
    fn pack_fail_protocol() {
        let s: &str = "bad:CA98593CB9DC0FA48B2BE01E53D042E22B47862D646F9F19E2889A7961663663";
        let err: PackHashError  = <str as TryPack<Hash<Blake3>>>::try_pack(s).expect_err("packing invalid protocol should fail");
        assert_eq!(err, PackHashError::BadProtocol);
    }

    #[test]
    fn pack_fail_hex() {
        let s: &str = "blake3:BAD!";
        let err: PackHashError  = <str as TryPack<Hash<Blake3>>>::try_pack(s).expect_err("packing invalid protocol should fail");
        assert!(matches!(err, PackHashError::BadHex(..)));
    }
}