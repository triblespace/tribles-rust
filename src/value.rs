pub mod schemas;

use crate::id::RawId;

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
pub struct Value<T: ValueSchema> {
    pub bytes: RawValue,
    _schema: PhantomData<T>,
}

impl<S: ValueSchema> Value<S> {
    pub fn new(value: RawValue) -> Self {
        Self {
            bytes: value,
            _schema: PhantomData,
        }
    }

    pub fn unpack<'a, T>(&'a self) -> T
    where
        T: UnpackValue<'a, S>,
    {
        <T as UnpackValue<'a, S>>::unpack(self)
    }

    pub fn try_unpack<'a, T>(&'a self) -> Result<T, <T as TryUnpackValue<S>>::Error>
    where
        T: TryUnpackValue<'a, S>,
    {
        <T as TryUnpackValue<'a, S>>::try_unpack(self)
    }
}

impl<T: ValueSchema> Copy for Value<T> {}

impl<T: ValueSchema> Clone for Value<T> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            _schema: PhantomData,
        }
    }
}

impl<T: ValueSchema> PartialEq for Value<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<T: ValueSchema> Eq for Value<T> {}

impl<T: ValueSchema> Hash for Value<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl<T: ValueSchema> Ord for Value<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl<T: ValueSchema> PartialOrd for Value<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ValueSchema> Debug for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Value<{}>({})",
            std::any::type_name::<T>(),
            ToHex::encode_hex::<String>(&self.bytes)
        )
    }
}

pub trait ValueSchema: Sized {
    const ID: RawId;

    fn pack<T: PackValue<Self> + ?Sized>(t: &T) -> Value<Self> {
        t.pack()
    }

    fn try_pack<T: TryPackValue<Self> + ?Sized>(
        t: &T,
    ) -> Result<Value<Self>, <T as TryPackValue<Self>>::Error> {
        t.try_pack()
    }
}

pub trait PackValue<S: ValueSchema> {
    fn pack(&self) -> Value<S>;
}
pub trait UnpackValue<'a, S: ValueSchema> {
    fn unpack(v: &'a Value<S>) -> Self;
}

pub trait TryPackValue<S: ValueSchema> {
    type Error;
    fn try_pack(&self) -> Result<Value<S>, Self::Error>;
}
pub trait TryUnpackValue<'a, S: ValueSchema>: Sized {
    type Error;
    fn try_unpack(v: &'a Value<S>) -> Result<Self, Self::Error>;
}
