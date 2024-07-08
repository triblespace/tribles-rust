use std::fmt;
use std::marker::PhantomData;

use digest::{typenum::U32, Digest};
use hex::ToHex;

use crate::types::Hash;

use crate::{Bloblike, Value, ValueParseError, Valuelike};

#[repr(transparent)]
pub struct Handle<H, T> {
    pub hash: Hash<H>,
    _type: PhantomData<T>,
}

impl<H, T> Copy for Handle<H, T> {}

impl<H, T> Clone for Handle<H, T> {
    fn clone(&self) -> Handle<H, T> {
        *self
    }
}

impl<H, T> PartialEq for Handle<H, T> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl<H, T> Eq for Handle<H, T> {}

impl<H, T> PartialOrd for Handle<H, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<H, T> Ord for Handle<H, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl<H, T> fmt::Debug for Handle<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Handle<{}, {}>({})",
            std::any::type_name::<H>(),
            std::any::type_name::<T>(),
            self.hash.bytes.encode_hex::<String>()
        )
    }
}

impl<H, T> Handle<H, T>
where
    T: Bloblike,
    H: Digest<OutputSize = U32>,
{
    pub unsafe fn new(hash: Hash<H>) -> Handle<H, T> {
        Handle {
            hash,
            _type: PhantomData,
        }
    }

    pub fn from(value: &T) -> Self {
        value.as_handle()
    }
}

impl<H, T> Valuelike for Handle<H, T> {
    fn from_value(value: Value) -> Result<Self, ValueParseError> {
        Ok(Handle {
            hash: Hash::new(value),
            _type: PhantomData,
        })
    }

    fn into_value(value: &Self) -> Value {
        value.hash.bytes
    }
}
