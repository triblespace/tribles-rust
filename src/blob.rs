use std::fmt::Debug;

use bytes::Bytes;

pub type Blob = Bytes;


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

#[derive(Debug)]
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
