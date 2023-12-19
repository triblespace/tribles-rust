use std::{fmt, marker::PhantomData};

use hex::ToHex;

use crate::types::{Value, Valuelike};

#[repr(transparent)]
pub struct Hash<H> {
    pub value: Value,
    _hasher: PhantomData<H>,
}

impl<H> Hash<H> {
    pub fn new(value: Value) -> Self {
        Hash {
            value,
            _hasher: PhantomData,
        }
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
        self.value == other.value
    }
}
impl<H> Eq for Hash<H> {}

impl<H> fmt::Debug for Hash<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hash<{}>({})",
            std::any::type_name::<H>(),
            self.value.encode_hex::<String>()
        )
    }
}

impl<H> Valuelike for Hash<H> {
    fn from_value(value: Value) -> Self {
        Hash::new(value)
    }

    fn into_value(&self) -> Value {
        self.value
    }
}

use blake2::{digest::typenum::U32, Blake2b as blake};
pub type Blake2b = blake<U32>;
