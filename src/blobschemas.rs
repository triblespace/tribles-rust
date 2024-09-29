//! This is a collection of Rust types that can be (de)serialized as
//! [Value]s, and [Blob]s.

pub mod packed;
pub mod simplearchive;
pub mod succinctarchive;

pub use simplearchive::SimpleArchive;
pub use succinctarchive::SuccinctArchive;

use crate::Blob;

pub trait BlobSchema: Sized {
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
impl BlobSchema for UnknownBlob {}
