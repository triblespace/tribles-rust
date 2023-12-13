use std::marker::PhantomData;

use digest::{typenum::U32, Digest, OutputSizeUser};

use crate::trible::{Blob, Value};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Handle<H, T> {
    pub hash: Value,
    _type: PhantomData<T>,
    _hasher: PhantomData<H>
}

impl<H, T> Handle<H, T> {
    pub fn new(value: Value) -> Handle<H, T> {
        Handle {
            hash: value,
            _type: PhantomData,
            _hasher: PhantomData
        }
    }
}

impl<H, T> From<Value> for Handle<H, T> {
    fn from(value: Value) -> Self {
        Handle {
            hash: value,
            _type: PhantomData,
            _hasher: PhantomData
        }
    }
}

impl<H, T> From<&Handle<H, T>> for Value {
    fn from(handle: &Handle<H, T>) -> Self {
        handle.hash
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
