use std::fmt;
use std::marker::PhantomData;

use hex::ToHex;
use digest::{typenum::U32, Digest, OutputSizeUser};

use crate::trible::{Blob, Value};
use crate::types::syntactic::Hash;

#[repr(transparent)]
pub struct Handle<H, T>
{
    pub hash: Hash<H>,
    _type: PhantomData<T>,
}

impl<H, T> Copy for Handle<H, T> { }

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

impl<H, T> fmt::Debug for Handle<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle<{}, {}>({})", std::any::type_name::<H>(), std::any::type_name::<T>(), self.hash.value.encode_hex::<String>())
    }
}

impl<H, T> Handle<H, T> {
    pub fn new(value: Value) -> Handle<H, T> {
        Handle {
            hash: Hash::new(value),
            _type: PhantomData,
        }
    }
}

impl<H, T> From<Value> for Handle<H, T> {
    fn from(value: Value) -> Self {
        Handle {
            hash: Hash::new(value),
            _type: PhantomData,
        }
    }
}

impl<H, T> From<&Handle<H, T>> for Value {
    fn from(handle: &Handle<H, T>) -> Self {
        handle.hash.value
    }
}


impl<H, T> From<&T> for Handle<H, T>
    where
    for<'a> &'a T: Into<Blob>,
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    fn from(value: &T) -> Self {
        let digest = H::digest(value.into());
        Handle::new(digest.into())
    }
}
