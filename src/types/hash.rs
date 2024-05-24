use std::{fmt, hash::Hasher, marker::PhantomData};

use digest::{Digest, OutputSizeUser};
use hex::ToHex;
use minibytes::Bytes;

use crate::{Value, ValueParseError, Valuelike};

#[repr(transparent)]
pub struct Hash<H> {
    pub bytes: Value,
    _hasher: PhantomData<H>,
}

impl<H> Hash<H> {
    pub fn new(bytes: Value) -> Self {
        Hash {
            bytes,
            _hasher: PhantomData,
        }
    }
}
impl<H> Hash<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    pub fn digest(blob: &Bytes) -> Self {
        Self::new(H::digest(&blob).into())
    }
}

impl<H> Copy for Hash<H> {}

impl<H> Clone for Hash<H> {
    fn clone(&self) -> Hash<H> {
        *self
    }
}

impl<H> PartialEq for Hash<H> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}
impl<H> Eq for Hash<H> {}

impl<H> std::hash::Hash for Hash<H> {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.bytes.hash(state);
    }
}

impl<H> fmt::Debug for Hash<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hash<{}>({})",
            std::any::type_name::<H>(),
            self.bytes.encode_hex::<String>()
        )
    }
}

impl<H> Valuelike for Hash<H> {
    fn from_value(bytes: Value) -> Result<Self, ValueParseError> {
        Ok(Hash::new(bytes))
    }

    fn into_value(hash: &Self) -> Value {
        hash.bytes
    }
}

use blake2::{digest::typenum::U32, Blake2b as Blake2bUnsized};
pub type Blake2b = Blake2bUnsized<U32>;

pub use blake3::Hasher as Blake3;
