use anybytes::Bytes;
use std::convert::Infallible;
use tribles::blob::{Blob, BlobSchema, ToBlob, TryFromBlob};
use tribles::id::{id_hex, Id};
use tribles::value::{FromValue, ToValue, Value, ValueSchema, VALUE_LEN};

// beginning

pub struct U64LE;

impl ValueSchema for U64LE {
    const VALUE_SCHEMA_ID: Id = id_hex!("0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A");
    type ValidationError = Infallible;
}

impl ToValue<U64LE> for u64 {
    fn to_value(self) -> Value<U64LE> {
        let mut raw = [0u8; VALUE_LEN];
        raw[..8].copy_from_slice(&self.to_le_bytes());
        Value::new(raw)
    }
}

impl FromValue<'_, U64LE> for u64 {
    fn from_value(v: &Value<U64LE>) -> Self {
        u64::from_le_bytes(v.raw[..8].try_into().unwrap())
    }
}

pub struct BytesBlob;

impl BlobSchema for BytesBlob {
    const BLOB_SCHEMA_ID: Id = id_hex!("B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0");
}

impl ToBlob<BytesBlob> for Bytes {
    fn to_blob(self) -> Blob<BytesBlob> {
        Blob::new(self)
    }
}

impl TryFromBlob<BytesBlob> for Bytes {
    type Error = Infallible;
    fn try_from_blob(b: Blob<BytesBlob>) -> Result<Self, Self::Error> {
        Ok(b.bytes)
    }
}

// ending

fn main() {}
