//! This is a collection of Rust types that can be (de)serialized as [crate::prelude::Blob]s.

pub mod longstring;
pub mod simplearchive;
pub mod succinctarchive;

use anybytes::Bytes;

use crate::blob::BlobSchema;
use crate::id::Id;
use crate::id_hex;

use super::{Blob, FromBlob, ToBlob};

/// A blob schema for an unknown blob.
/// This blob schema is used as a fallback when the blob schema is not known.
/// It is not recommended to use this blob schema in practice.
/// Instead, use a specific blob schema.
///
/// Any bit pattern can be a valid blob of this schema.
pub struct UnknownBlob;
impl BlobSchema for UnknownBlob {
    const BLOB_SCHEMA_ID: Id = id_hex!("EAB14005141181B0C10C4B5DD7985F8D");
}

impl FromBlob<'_, UnknownBlob> for Bytes {
    fn from_blob(blob: &Blob<UnknownBlob>) -> Self {
        blob.bytes.clone()
    }
}

impl ToBlob<UnknownBlob> for Bytes {
    fn to_blob(self) -> Blob<UnknownBlob> {
        Blob::new(self)
    }
}
