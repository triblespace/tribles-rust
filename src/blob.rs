use std::fmt::Debug;

use bytes::Bytes;
use digest::{consts::U32, Digest, OutputSizeUser};

use crate::{types::Hash, Handle};

/// A type that is convertible to and from a [Blob].
pub trait Bloblike<'a>: Sized {
    type Read: 'a;

    fn from_blob(blob: &'a Bytes) -> Result<Self::Read, BlobParseError>;
    fn into_blob(self) -> Bytes;
    fn as_handle<H>(&self) -> Handle<H, Self>
    where
        H: Digest + OutputSizeUser<OutputSize = U32>;
}

impl<'a> Bloblike<'a> for Bytes {
    type Read = &'a Bytes;
    fn from_blob(blob: &'a Bytes) -> Result<Self::Read, BlobParseError> {
        Ok(blob)
    }

    fn into_blob(self) -> Bytes {
        self
    }
    fn as_handle<H>(&self) -> Handle<H, Self>
    where
        H: Digest + OutputSizeUser<OutputSize = U32>,
    {
        let digest = H::digest(self);
        unsafe { Handle::new(Hash::new(digest.into())) }
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
