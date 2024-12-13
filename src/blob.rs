pub mod schemas;

use crate::{
    id::Id,
    value::{
        schemas::hash::{Handle, HashProtocol},
        Value, ValueSchema,
    },
};

use std::{
    fmt::{self, Debug},
    hash::Hash,
    marker::PhantomData,
};

pub use anybytes::Bytes;

#[repr(transparent)]
pub struct Blob<S: BlobSchema> {
    pub bytes: Bytes,
    _schema: PhantomData<S>,
}

impl<S: BlobSchema> Blob<S> {
    pub fn new(bytes: Bytes) -> Self {
        Self {
            bytes,
            _schema: PhantomData,
        }
    }

    pub fn transmute<T: BlobSchema>(&self) -> &Blob<T> {
        unsafe { std::mem::transmute(self) }
    }

    pub fn as_handle<H>(&self) -> Value<Handle<H, S>>
    where
        H: HashProtocol,
        Handle<H, S>: ValueSchema,
    {
        let digest = H::digest(&self.bytes);
        Value::new(digest.into())
    }

    pub fn from_blob<'a, T>(&'a self) -> T
    where
        T: FromBlob<'a, S>,
    {
        <T as FromBlob<'a, S>>::from_blob(self)
    }

    pub fn try_from_blob<'a, T>(&'a self) -> Result<T, <T as TryFromBlob<'a, S>>::Error>
    where
        T: TryFromBlob<'a, S>,
    {
        <T as TryFromBlob<'a, S>>::try_from_blob(self)
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
        write!(f, "Blob<{}>", std::any::type_name::<T>())
    }
}

pub trait BlobSchema: Sized + 'static {
    const BLOB_SCHEMA_ID: Id;

    fn to_blob<T: ToBlob<Self>>(t: T) -> Blob<Self> {
        t.to_blob()
    }

    fn try_to_blob<T: TryToBlob<Self>>(t: T) -> Result<Blob<Self>, <T as TryToBlob<Self>>::Error> {
        t.try_to_blob()
    }
}

pub trait ToBlob<S: BlobSchema> {
    fn to_blob(self) -> Blob<S>;
}
pub trait FromBlob<'a, S: BlobSchema> {
    fn from_blob(b: &'a Blob<S>) -> Self;
}

pub trait TryToBlob<S: BlobSchema> {
    type Error;
    fn try_to_blob(&self) -> Result<Blob<S>, Self::Error>;
}

pub trait TryFromBlob<'a, S: BlobSchema>: Sized {
    type Error;
    fn try_from_blob(b: &'a Blob<S>) -> Result<Self, Self::Error>;
}

impl<S: BlobSchema> ToBlob<S> for Blob<S> {
    fn to_blob(self) -> Blob<S> {
        self
    }
}

impl<'a, S: BlobSchema> FromBlob<'a, S> for Blob<S> {
    fn from_blob(b: &'a Blob<S>) -> Self {
        b.clone()
    }
}
