use std::marker::PhantomData;

use digest::{Digest, typenum::U32};
use anybytes::Bytes;

use crate::{ Value, Schema };

pub struct Hash<H> {
    _hasher: PhantomData<H>,
}

impl<H> Schema for Hash<H> {}

impl<H> Hash<H>
where
    H: Digest<OutputSize = U32>,
{
    pub fn digest(blob: &Bytes) -> Value<Self> {
        Value::new(H::digest(&blob).into())
    }
}

use blake2::Blake2b as Blake2bUnsized;
pub type Blake2b = Blake2bUnsized<U32>;

pub use blake3::Hasher as Blake3;
