use std::fmt;
use std::marker::PhantomData;

use digest::{typenum::U32, Digest, OutputSizeUser};
use hex::ToHex;

use crate::types::syntactic::Hash;
use crate::types::{Blob, Value};

use super::{Bloblike, ValueParseError, Valuelike};

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

impl<H, T> fmt::Debug for Handle<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Handle<{}, {}>({})",
            std::any::type_name::<H>(),
            std::any::type_name::<T>(),
            self.hash.value.encode_hex::<String>()
        )
    }
}

impl<H, T> Handle<H, T> {
    pub unsafe fn new(hash: Hash<H>) -> Handle<H, T> {
        Handle {
            hash,
            _type: PhantomData,
        }
    }
}

impl<H, T> From<&T> for Handle<H, T>
where
    T: Bloblike,
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    fn from(value: &T) -> Self {
        let blob: Blob = value.into_blob();
        let digest = H::digest(blob);
        unsafe {Handle::new(Hash::new(digest.into()))}
    }
}

impl<H, T> Valuelike for Handle<H, T> {
    fn from_value(value: Value) -> Result<Self, ValueParseError> {
        Ok(Handle {
            hash: Hash::new(value),
            _type: PhantomData,
        })
    }

    fn into_value(&self) -> Value {
        self.hash.value
    }
}
