use core::fmt;
use std::{cmp::Ordering, fmt::Debug, hash::Hash, marker::PhantomData};

use hex::ToHex;

use crate::{schemas::{TryUnpack, Unpack}, Schema};

pub const VALUE_LEN: usize = 32;
pub type RawValue = [u8; VALUE_LEN];

// Idea: We could also have a Raw<T> type that could
// be validated with `T::validate(Raw<T>) -> Value<T>`
// in order to make sure that Value always has a valid bit pattern for type T.
// Queries would then for example return `Raw<T>`.
// But this would also make things more complicated and put a lot of focus
// on the (hopefully) rare cases where values contain bad/wrong data.
// Also we might want to make use of the fact that we can ignore malformed cases
// if we just want to annotate metadata or do statistical analysis for example.

#[repr(transparent)]
pub struct Value<T: Schema>{
    pub bytes: RawValue,
    _schema: PhantomData<T>,
}

impl<S: Schema> Value<S> {
    pub fn new(value: RawValue) -> Self {
        Self {
            bytes: value,
            _schema: PhantomData
        }
    }

    pub fn unpack<'a, T>(&'a self) -> T
    where T: Unpack<'a, S> {
        <T as Unpack<'a, S>>::unpack(self)
    }

    pub fn try_unpack<'a, T>(&'a self) -> Result<T, <T as TryUnpack<S>>::Error>
    where T: TryUnpack<'a, S> {
        <T as TryUnpack<'a, S>>::try_unpack(self)
    }
}

impl<T: Schema> Copy for Value<T> {}

impl<T: Schema> Clone for Value<T> {
    fn clone(&self) -> Self {
        Self { bytes: self.bytes.clone(), _schema: PhantomData }
    }
}

impl<T: Schema> PartialEq for Value<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<T: Schema> Eq for Value<T> {}

impl<T: Schema> Hash for Value<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl<T: Schema> Ord for Value<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl<T: Schema> PartialOrd for Value<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Schema> Debug for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Value<{}>({})",
            std::any::type_name::<T>(),
            ToHex::encode_hex::<String>(&self.bytes)
        )
    }
}
