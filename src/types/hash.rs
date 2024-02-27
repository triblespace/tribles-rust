use std::{fmt, hash::Hasher, marker::PhantomData};

use hex::ToHex;

use crate::{Value, ValueParseError, Valuelike};

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

impl<H> std::hash::Hash for Hash<H> {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.value.hash(state);
    }
}


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
    fn from_value(value: Value) -> Result<Self, ValueParseError> {
        Ok(Hash::new(value))
    }

    fn into_value(value: &Self) -> Value {
        value.value
    }
}

use blake2::{digest::typenum::U32, Blake2b as Blake2bUnsized};
pub type Blake2b = Blake2bUnsized<U32>;

pub use blake3::Hasher as Blake3;
