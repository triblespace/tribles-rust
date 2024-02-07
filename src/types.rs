//! This is a collection of Rust types that can be (de)serialized as
//! [Id]s, [Value]s, and [Blob]s.

pub mod handle;
pub mod semantic;
pub mod syntactic;

use std::{convert::TryInto, fmt::Debug, sync::Arc};

pub type Id = [u8; 16];
pub type Value = [u8; 32];
pub type Blob = Arc<[u8]>;

pub const ID_LEN: usize = 16;
pub const VALUE_LEN: usize = 32;

/// A type that is convertible to and from an [Id].
/// Must also provide a method to generate new unique values of that type.
pub trait Idlike {
    fn from_id(id: Id) -> Self;
    fn into_id(&self) -> Id;
    fn factory() -> Self;
}

/// A type that is convertible to and from a [Value].
pub trait Valuelike: Sized {
    fn from_value(value: Value) -> Result<Self, ValueParseError>;
    fn into_value(&self) -> Value;
}

/// A type that is convertible to and from a [Blob].
pub trait Bloblike: Sized {
    fn from_blob(blob: Blob) -> Result<Self, BlobParseError>;
    fn into_blob(&self) -> Blob;
}

impl Bloblike for Blob {
    fn from_blob(blob: Blob) -> Result<Self, BlobParseError> {
        Ok(blob)
    }

    fn into_blob(&self) -> Blob {
        self.clone()
    }
}

pub fn id_into_value(id: Id) -> Value {
    let mut data = [0; VALUE_LEN];
    data[16..=31].copy_from_slice(&id[..]);
    data
}

impl<T: Idlike> Valuelike for T {
    fn from_value(value: Value) -> Result<Self, ValueParseError> {
        Ok(Self::from_id(value[16..32].try_into().unwrap()))
    }

    fn into_value(&self) -> Value {
        id_into_value(self.into_id())
    }
}

pub struct ValueParseError {
    value: Value,
    msg: String,
}

impl ValueParseError {
    pub fn new(value: Value, msg: &str) -> Self {
        ValueParseError {
            value,
            msg: msg.to_owned(),
        }
    }
}

impl Eq for ValueParseError {}
impl PartialEq for ValueParseError {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.msg == other.msg
    }
}
impl Debug for ValueParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueParseError")
            .field("value", &hex::encode(&self.value))
            .field("msg", &self.msg)
            .finish()
    }
}

pub struct BlobParseError {
    blob: Blob,
    msg: String,
}

impl BlobParseError {
    pub fn new(blob: Blob, msg: &str) -> Self {
        BlobParseError {
            blob,
            msg: msg.to_owned(),
        }
    }
}

impl Eq for BlobParseError {}
impl PartialEq for BlobParseError {
    fn eq(&self, other: &Self) -> bool {
        self.blob == other.blob && self.msg == other.msg
    }
}
