//! This is a collection of Rust types that can be (de)serialized as [Blob]s.

pub mod longstring;
pub mod simplearchive;
pub mod succinctarchive;

use hex_literal::hex;
pub use simplearchive::SimpleArchive;
pub use succinctarchive::SuccinctArchive;

use crate::{Blob, RawId};

pub trait BlobSchema: Sized {
    const ID: RawId;

    fn pack<T: PackBlob<Self> + ?Sized>(t: &T) -> Blob<Self> {
        t.pack()
    }

    fn try_pack<T: TryPackBlob<Self> + ?Sized>(
        t: &T,
    ) -> Result<Blob<Self>, <T as TryPackBlob<Self>>::Error> {
        t.try_pack()
    }
}

pub trait PackBlob<S: BlobSchema> {
    fn pack(&self) -> Blob<S>;
}
pub trait UnpackBlob<'a, S: BlobSchema> {
    fn unpack(b: &'a Blob<S>) -> Self;
}

pub trait TryPackBlob<S: BlobSchema> {
    type Error;
    fn try_pack(&self) -> Result<Blob<S>, Self::Error>;
}

pub trait TryUnpackBlob<'a, S: BlobSchema>: Sized {
    type Error;
    fn try_unpack(b: &'a Blob<S>) -> Result<Self, Self::Error>;
}

pub struct UnknownBlob;
impl BlobSchema for UnknownBlob { const ID: RawId = hex!("EAB14005141181B0C10C4B5DD7985F8D");}
