use std::{marker::PhantomData, fmt};

use hex::ToHex;

use crate::{trible::*, types::{ToValue, FromValue}};

#[repr(transparent)]
pub struct Hash<H>
{
    pub value: Value,
    _hasher: PhantomData<H>
}

impl<H> Hash<H> {
    pub fn new(value: Value) -> Self {
        Hash {
            value,
            _hasher: PhantomData
        }
    }
}

impl<H> Copy for Hash<H> { }

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
        write!(f, "Hash<{}>({})", std::any::type_name::<H>(), self.value.encode_hex::<String>())
    }
}

// TODO? Rework macro to proc macro and automatically detect generics?
impl<H> FromValue for Hash<H>
where
    Hash<H>: From<Value>,
{
    type Rep = Hash<H>;

    fn from_value(value: Value) -> Self::Rep {
        value.into()
    }
}

impl<H> ToValue for Hash<H>
where
    for<'a> &'a Hash<H>: Into<Value>,
{
    type Rep = Hash<H>;

    fn to_value(value: &Self::Rep) -> Value {
        value.into()
    }
}

impl<H> From<&Hash<H>> for Value {
    fn from(hash: &Hash<H>) -> Self {
        hash.value
    }
}

impl<H> From<Value> for Hash<H> {
    fn from(value: Value) -> Self {
        Hash::new(value)
    }
}
