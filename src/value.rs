use core::fmt;
use std::{fmt::Debug, hash::Hash, marker::PhantomData};

use hex::ToHex;

pub const VALUE_LEN: usize = 32;
pub type RawValue = [u8; VALUE_LEN];

pub struct Value<T>{
    pub bytes: RawValue,
    _type: PhantomData<T>,
}

impl<T> Value<T> {
    pub fn new(value: RawValue) -> Self {
        Self {
            bytes: value,
            _type: PhantomData
        }
    }
}

impl<T> Copy for Value<T> {}

impl<T> Clone for Value<T> {
    fn clone(&self) -> Self {
        Self { bytes: self.bytes.clone(), _type: PhantomData }
    }
}

impl<T> PartialEq for Value<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<T> Eq for Value<T> {}

impl<T> Hash for Value<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl<T> Debug for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Value<{}>({})",
            std::any::type_name::<T>(),
            ToHex::encode_hex::<String>(&self.bytes)
        )
    }
}
