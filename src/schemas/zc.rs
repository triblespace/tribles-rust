use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anybytes::{ByteOwner, Bytes};
use digest::{consts::U32, Digest};
use zerocopy::FromBytes;

use crate::{schemas::Handle, BlobParseError, Bloblike, Value};

pub struct ZC<T> {
    bytes: Bytes,
    _type: PhantomData<T>,
}

impl<T> Clone for ZC<T> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            _type: PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for ZC<T>
where
    T: FromBytes + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner: &T = self;
        Debug::fmt(inner, f)
    }
}

impl<T> std::ops::Deref for ZC<T>
where
    T: FromBytes,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        FromBytes::ref_from(&self.bytes).expect("ZeroCopy validation should happen at creation")
    }
}

impl<T> AsRef<[u8]> for ZC<T>
where
    T: FromBytes,
{
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<T> From<T> for ZC<T>
where
    T: ByteOwner,
{
    fn from(value: T) -> Self {
        ZC {
            bytes: Bytes::from_owner(value),
            _type: PhantomData,
        }
    }
}

impl<T> From<Arc<T>> for ZC<T>
where
    T: ByteOwner,
{
    fn from(value: Arc<T>) -> Self {
        ZC {
            bytes: Bytes::from_arc(value),
            _type: PhantomData,
        }
    }
}

impl<T> Bloblike for ZC<T>
where
    T: FromBytes,
{
    fn into_blob(self) -> Bytes {
        self.bytes
    }

    fn from_blob(blob: Bytes) -> Result<Self, BlobParseError> {
        if <T as FromBytes>::ref_from(&blob).is_none() {
            Err(BlobParseError::new(
                "wrong size or alignment of bytes for type",
            ))
        } else {
            Ok(ZC {
                bytes: blob,
                _type: PhantomData,
            })
        }
    }

    fn as_handle<H>(&self) -> Value<Handle<H, Self>>
    where
        H: Digest<OutputSize = U32>,
    {
        let digest = H::digest(&self.bytes);
        Value::new(digest.into())
    }
}
