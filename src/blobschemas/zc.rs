use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anybytes::{ByteOwner, Bytes};
use zerocopy::FromBytes;

use crate::Blob;

use super::{BlobSchema, PackBlob, TryUnpackBlob};

pub struct ZC<T> {
    bytes: Bytes,
    _type: PhantomData<T>,
}

impl<T> BlobSchema for ZC<T> {}

impl<T> PackBlob<ZC<T>> for ZC<T> {
    fn pack(&self) -> crate::Blob<ZC<T>> {
        Blob::new(self.bytes.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZCUnpackError {
    BadLayout
}

impl<'a, T> TryUnpackBlob<'a, ZC<T>> for ZC<T>
where T: FromBytes {
    type Error = ZCUnpackError;

    fn try_unpack(b: &'a Blob<ZC<T>>) -> Result<Self, Self::Error> {
        if <T as FromBytes>::ref_from(&b.bytes).is_none() {
            Err(ZCUnpackError::BadLayout)
        } else {
            Ok(ZC {
                bytes: b.bytes.clone(),
                _type: PhantomData,
            })
        }
    }
}

impl<'a, T> TryUnpackBlob<'a, ZC<T>> for &'a T
where T: FromBytes {
    type Error = ZCUnpackError;

    fn try_unpack(b: &'a Blob<ZC<T>>) -> Result<Self, Self::Error> {
        match <T as FromBytes>::ref_from(&b.bytes) {
            Some(r) => Ok(r),
            None => Err(ZCUnpackError::BadLayout)
        }
    }
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
