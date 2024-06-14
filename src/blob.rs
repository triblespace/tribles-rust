use std::fmt::Debug;
use digest::{consts::U32, Digest};

use crate::{types::Hash, Handle};

pub use minibytes::Bytes;

/// A type that is convertible to and from a [Blob].
pub trait Bloblike: Sized {
    fn into_blob(self) -> Bytes;
    fn from_blob(blob: Bytes) -> Result<Self, BlobParseError>;
    fn as_handle<H>(&self) -> Handle<H, Self>
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

    fn as_handle<H>(&self) -> Handle<H, Self>
    where
        H: Digest<OutputSize = U32>,
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
