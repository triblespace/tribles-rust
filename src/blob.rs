use digest::{consts::U32, Digest};
use std::{fmt::Debug, marker::PhantomData};

use crate::{schemas::Handle, Value};

pub use anybytes::Bytes;

/*
#[repr(transparent)]
pub struct Blob<T: BlobSchema> {
    pub bytes: Bytes,
    _schema: PhantomData<T>,
}
*/
pub trait Bloblike: Sized {
    fn into_blob(self) -> Bytes;
    fn from_blob(blob: Bytes) -> Result<Self, BlobParseError>;
    fn as_handle<H>(&self) -> Value<Handle<H, Self>>
    where
        H: Digest<OutputSize = U32>;
}

impl<'a> Bloblike for Bytes {
    fn into_blob(self) -> Bytes {
        self
    }

    fn from_blob(blob: Bytes) -> Result<Self, BlobParseError> {
        Ok(blob)
    }

    fn as_handle<H>(&self) -> Value<Handle<H, Self>>
    where
        H: Digest<OutputSize = U32>,
    {
        let digest = H::digest(self);
        Value::new(digest.into())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlobParseError {
    msg: String,
}

impl BlobParseError {
    pub fn new(msg: &str) -> Self {
        BlobParseError {
            msg: msg.to_owned(),
        }
    }
}
