use core::fmt;
use std::{cmp::Ordering, fmt::Debug, hash::Hash, marker::PhantomData};

use hex::ToHex;

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

impl<T> Ord for Value<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl<T> PartialOrd for Value<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
