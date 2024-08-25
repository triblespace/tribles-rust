use digest::{consts::U32, Digest};
use std::{fmt::{self, Debug}, hash::Hash, marker::PhantomData};

use crate::{blobschemas::{BlobSchema, TryUnpackBlob, UnpackBlob}, valueschemas::Handle, Value};

pub use anybytes::Bytes;

#[repr(transparent)]
pub struct Blob<T: BlobSchema> {
    pub bytes: Bytes,
    _schema: PhantomData<T>,
}

impl<S: BlobSchema> Blob<S> {
    pub fn new(bytes: Bytes) -> Self {
        Self {
            bytes,
            _schema: PhantomData,
        }
    }

    pub fn as_handle<H>(&self) -> Value<Handle<H, S>>
    where
        H: Digest<OutputSize = U32>,
    {
        let digest = H::digest(&self.bytes);
        Value::new(digest.into())
    }

    pub fn unpack<'a, T>(&'a self) -> T
    where
        T: UnpackBlob<'a, S>,
    {
        <T as UnpackBlob<'a, S>>::unpack(self)
    }

    pub fn try_unpack<'a, T>(&'a self) -> Result<T, <T as TryUnpackBlob<S>>::Error>
    where
        T: TryUnpackBlob<'a, S>,
    {
        <T as TryUnpackBlob<'a, S>>::try_unpack(self)
    }
}

impl<T: BlobSchema> Clone for Blob<T> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            _schema: PhantomData,
        }
    }
}

impl<T: BlobSchema> PartialEq for Blob<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<T: BlobSchema> Eq for Blob<T> {}

impl<T: BlobSchema> Hash for Blob<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl<T: BlobSchema> Debug for Blob<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Blob<{}>",
            std::any::type_name::<T>()
        )
    }
}
