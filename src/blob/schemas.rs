//! This is a collection of Rust types that can be (de)serialized as [crate::prelude::Blob]s.

pub mod longstring;
pub mod simplearchive;
pub mod succinctarchive;

use crate::blob::BlobSchema;
use crate::id::Id;
use crate::id_hex;

pub struct UnknownBlob;
impl BlobSchema for UnknownBlob {
    const BLOB_SCHEMA_ID: Id = id_hex!("EAB14005141181B0C10C4B5DD7985F8D");
}
