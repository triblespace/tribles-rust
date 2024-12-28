//! Anything that can be represented as a byte sequence.
//!
//! A blob is a immutable sequence of bytes that can be used to represent any kind of data.
//! It is the fundamental building block of data storage and transmission.
//! It also provides the `BlobSchema` trait, which is used to define the schema of a blob.
//!
//! This is similar to the `Value` type and the `ValueSchema` trait in the [`value`](crate::value) module.
//! But while values (and tribles) are used "in the small" to represent individual data items,
//! blobs are used "in the large" to represent larger data structures like files, images, videos, etc.,
//! collections of data items, or even entire databases.
//!
//! # Example
//!
//! ```
//! use tribles::prelude::*;
//! use tribles::examples::literature;
//! use tribles::remote::commit::commits;
//! use valueschemas::{Handle, Blake3};
//! use blobschemas::{SimpleArchive, LongString};
//!
//! // Let's build a BlobSet and fill it with some data.
//! // Note that we are using the Blake3 hash protocol here.
//! let mut blobset: BlobSet<Blake3> = BlobSet::new();
//!
//! let author_id = ufoid();
//!
//! let quote_a: Value<Handle<Blake3, LongString>> = blobset.insert("Deep in the human unconscious is a pervasive need for a logical universe that makes sense. But the real universe is always one step beyond logic.");
//! // Note how the type is inferred from it's usage in the [entity!](crate::namespace::entity!) macro.
//! let quote_b = blobset.insert("I must not fear. Fear is the mind-killer. Fear is the little-death that brings total obliteration. I will face my fear. I will permit it to pass over me and
//!  through me. And when it has gone past I will turn the inner eye to see its path. Where the fear has gone there will be nothing. Only I will remain.");
//!
//! let set = literature::entity!({
//!    title: "Dune",
//!    author: &author_id,
//!    quote: quote_a,
//!    quote: quote_b
//! });
//!
//! // Now we can serialize the TribleSet and store it in the BlobSet too.
//! let archived_set: Value<Handle<Blake3, SimpleArchive>> = blobset.insert(&set);
//!
//! // And store the handle in another TribleSet.
//! let meta_set = commits::entity!({
//!    tribles: archived_set,
//!    authored_by: ufoid(),
//!    short_message: "Initial commit"
//! });
//! ```

mod blobset;
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

pub use blobset::BlobSet;

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
