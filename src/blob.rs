//! Anything that can be represented as a byte sequence.
//!
//! A blob is a immutable sequence of bytes that can be used to represent any kind of data.
//! It is the fundamental building block of data storage and transmission.
//! The `BlobSchema` trait is used to define the abstract schema type of a blob.
//! This is similar to the `Value` type and the `ValueSchema` trait in the [`value`](crate::value) module.
//!
//! But while values (and tribles) are used "in the small" to represent individual data items,
//! blobs are used "in the large" to represent larger data structures like files, images, videos, etc.,
//! collections of data items, or even entire databases.
//!
//! # Example
//!
//! ```
//! use tribles::prelude::*;
//! use tribles::examples::literature;
//! use tribles::repo::repo;
//! use valueschemas::{Handle, Blake3};
//! use blobschemas::{SimpleArchive, LongString};
//! use rand::rngs::OsRng;
//! use ed25519_dalek::{Signature, Signer, SigningKey};
//!
//! // Let's build a BlobSet and fill it with some data.
//! // Note that we are using the Blake3 hash protocol here.
//! let mut memory_store: MemoryBlobStore<Blake3> = MemoryBlobStore::new();
//!
//! let book_author_id = fucid();
//! let quote_a: Value<Handle<Blake3, LongString>> = memory_store.put_blob("Deep in the human unconscious is a pervasive need for a logical universe that makes sense. But the real universe is always one step beyond logic.").unwrap();
//! // Note how the type is inferred from it's usage in the [entity!](crate::namespace::entity!) macro.
//! let quote_b = memory_store.put_blob("I must not fear. Fear is the mind-killer. Fear is the little-death that brings total obliteration. I will face my fear. I will permit it to pass over me and
//!  through me. And when it has gone past I will turn the inner eye to see its path. Where the fear has gone there will be nothing. Only I will remain.").unwrap();
//!
//! let set = literature::entity!({
//!    title: "Dune",
//!    author: &book_author_id,
//!    quote: quote_a,
//!    quote: quote_b
//! });
//!
//! // Now we can serialize the TribleSet and store it in the BlobSet too.
//! let archived_set_handle: Value<Handle<Blake3, SimpleArchive>> = memory_store.put_blob(&set).unwrap();
//!
//! let mut csprng = OsRng;
//! let commit_author_key: SigningKey = SigningKey::generate(&mut csprng);
//! let signature: Signature = commit_author_key.sign(&memory_store.reader().get_blob(archived_set_handle).unwrap().bytes);
//!
//! // And store the handle in another TribleSet.
//! let meta_set = repo::entity!({
//!    content: archived_set_handle,
//!    short_message: "Initial commit",
//!    signed_by: commit_author_key.verifying_key(),
//!    signature_r: signature,
//!    signature_s: signature,
//! });
//! ```

mod memoryblobstore;
pub mod schemas;

use crate::{
    id::Id, value::{
        schemas::hash::{Handle, HashProtocol},
        Value, ValueSchema,
    }
};

use std::{
    fmt::{self, Debug},
    hash::Hash,
    marker::PhantomData,
};

pub use memoryblobstore::MemoryBlobStore;

pub use anybytes::Bytes;

/// A blob is a immutable sequence of bytes that can be used to represent any kind of data.
/// It is the fundamental building block of data storage and transmission.
/// The `BlobSchema` type parameter is used to define the abstract schema type of a blob.
/// This is similar to the `Value` type and the `ValueSchema` trait in the [`value`](crate::value) module.
#[repr(transparent)]
pub struct Blob<S: BlobSchema> {
    pub bytes: Bytes,
    _schema: PhantomData<S>,
}

impl<S: BlobSchema> Blob<S> {
    /// Creates a new blob from a sequence of bytes.
    /// The bytes are stored in the blob as-is.
    pub fn new(bytes: Bytes) -> Self {
        Self {
            bytes,
            _schema: PhantomData,
        }
    }

    pub fn transmute<T: BlobSchema>(self) -> Blob<T> {
        Blob {
            bytes: self.bytes,
            _schema: PhantomData,
        }
    }

    /// Transmutes the blob to a blob of a different schema.
    /// This is a zero-cost operation.
    /// If the schema types are not compatible, this will not cause undefined behavior,
    /// but it might cause unexpected results.
    ///
    /// This is primarily used to give blobs with an [UnknownBlob](crate::blob::schemas::UnknownBlob) schema a more specific schema.
    /// Use with caution.
    pub fn as_transmute<T: BlobSchema>(&self) -> &Blob<T> {
        unsafe { std::mem::transmute(self) }
    }

    //TODO: Do we want to cache the handle somewhere so that we don't have to compute the hash every time?
    // We could use WeakBytes for this, but it would require one hash-map per HashProtocol.

    /// Hashes the blob with the given hash protocol and returns the hash as a handle.
    pub fn get_handle<H>(&self) -> Value<Handle<H, S>>
    where
        H: HashProtocol,
        Handle<H, S>: ValueSchema,
    {
        let digest = H::digest(&self.bytes);
        Value::new(digest.into())
    }

    /// Converts the blob to a concrete Rust type.
    /// If the conversion fails, this might cause a panic.
    /// Use [try_from_blob](Blob::try_from_blob) to explicitly handle the error.
    pub fn from_blob<T>(self) -> T
    where
        T: FromBlob<S>,
    {
        <T as FromBlob<S>>::from_blob(self)
    }

    /// Tries to convert the blob to a concrete Rust type.
    /// If the conversion fails, an error is returned.
    /// Use [from_blob](Blob::from_blob) if you are sure that the conversion will succeed.
    pub fn try_from_blob<T>(self) -> Result<T, <T as TryFromBlob<S>>::Error>
    where
        T: TryFromBlob<S>,
    {
        <T as TryFromBlob<S>>::try_from_blob(self)
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

/// A trait for defining the abstract schema type of a blob.
/// This is similar to the `ValueSchema` trait in the [`value`](crate::value) module.
pub trait BlobSchema: Sized + 'static {
    const BLOB_SCHEMA_ID: Id;

    /// Converts a concrete Rust type to a blob with this schema.
    /// If the conversion fails, this might cause a panic.
    /// Use [try_blob_from](BlobSchema::try_blob_from) to explicitly handle the error.
    fn blob_from<T: ToBlob<Self>>(t: T) -> Blob<Self> {
        t.to_blob()
    }

    /// Converts a concrete Rust type to a blob with this schema.
    /// If the conversion fails, an error is returned.
    /// Use [blob_from](BlobSchema::blob_from) if you are sure that the conversion will succeed.
    fn try_blob_from<T: TryToBlob<Self>>(
        t: T,
    ) -> Result<Blob<Self>, <T as TryToBlob<Self>>::Error> {
        t.try_to_blob()
    }
}

/// A trait for converting a Rust type to a [Blob] with a specific schema.
/// This trait is implemented on the concrete Rust type.
///
/// This might cause a panic if the conversion is not possible,
/// see [TryToBlob] for a conversion that returns a result.
///
/// This is the counterpart to the [FromBlob] trait.
///
/// See [ToValue](crate::value::ToValue) for the counterpart trait for values.
pub trait ToBlob<S: BlobSchema> {
    fn to_blob(self) -> Blob<S>;
}

/// A trait for converting a [Blob] with a specific schema to a Rust type.
/// This trait is implemented on the concrete Rust type.
///
/// This might cause a panic if the conversion is not possible,
/// see [TryFromBlob] for a conversion that returns a result.
///
/// This is the counterpart to the [ToBlob] trait.
///
/// See [FromValue](crate::value::FromValue) for the counterpart trait for values.
pub trait FromBlob<S: BlobSchema> {
    fn from_blob(b: Blob<S>) -> Self;
}

/// A trait for converting a Rust type to a [Blob] with a specific schema.
/// This trait is implemented on the concrete Rust type.
///
/// This might return an error if the conversion is not possible,
/// see [ToBlob] for cases where the conversion is guaranteed to succeed (or panic).
///
/// This is the counterpart to the [TryFromBlob] trait.
///
/// See [TryToValue](crate::value::TryToValue) for the counterpart trait for values.
pub trait TryToBlob<S: BlobSchema> {
    type Error;
    fn try_to_blob(&self) -> Result<Blob<S>, Self::Error>;
}

/// A trait for converting a [Blob] with a specific schema to a Rust type.
/// This trait is implemented on the concrete Rust type.
///
/// This might return an error if the conversion is not possible,
/// see [FromBlob] for cases where the conversion is guaranteed to succeed (or panic).
///
/// This is the counterpart to the [TryToBlob] trait.
///
/// See [TryFromValue](crate::value::TryFromValue) for the counterpart trait for values.
pub trait TryFromBlob<S: BlobSchema>: Sized {
    type Error;
    fn try_from_blob(b: Blob<S>) -> Result<Self, Self::Error>;
}

impl<S: BlobSchema> ToBlob<S> for Blob<S> {
    fn to_blob(self) -> Blob<S> {
        self
    }
}

impl<S: BlobSchema> FromBlob<S> for Blob<S> {
    fn from_blob(b: Blob<S>) -> Self {
        b
    }
}
