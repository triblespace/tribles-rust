//! This is a collection of Rust types that can be (de)serialized as [Blob]s.

use crate::blob::BlobSchema;
use crate::id::RawId;

use hex_literal::hex;

pub mod longstring;
pub mod simplearchive;
pub mod succinctarchive;

pub struct UnknownBlob;
impl BlobSchema for UnknownBlob {
    const ID: RawId = hex!("EAB14005141181B0C10C4B5DD7985F8D");
}
