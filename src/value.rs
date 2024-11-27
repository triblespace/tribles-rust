pub mod schemas;

use crate::id::RawId;

use core::fmt;
use std::{borrow::Borrow, cmp::Ordering, fmt::Debug, hash::Hash, marker::PhantomData};

use hex::{ToHex, FromHex, FromHexError};


pub const VALUE_LEN: usize = 32;
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C, align(32))]
pub struct RawValue {
    pub bytes: [u8; VALUE_LEN]
}

impl RawValue {
    pub fn new(bytes: &[u8; VALUE_LEN]) -> Self {
        Self {
            bytes: *bytes
        }
    }

    pub fn from_hex(hex: &str) -> Result<Self, FromHexError> {
        <[u8; 32]>::from_hex(hex).map(|bytes| Self {bytes})
    }
}

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
    pub raw: RawValue,
    _schema: PhantomData<T>,
}

impl<S: ValueSchema> Value<S> {
    pub fn new(bytes: &[u8; VALUE_LEN]) -> Self {
        Self {
            raw: RawValue::new(bytes),
            _schema: PhantomData,
        }
    }

    pub fn transmute_raw(value: &RawValue) -> &Self {
        unsafe { std::mem::transmute(value) }
    }

    pub fn from_value<'a, T>(&'a self) -> T
    where
        T: FromValue<'a, S>,
    {
        <T as FromValue<'a, S>>::from_value(self)
    }

    pub fn try_from_value<'a, T>(&'a self) -> Result<T, <T as TryFromValue<S>>::Error>
    where
        T: TryFromValue<'a, S>,
    {
        <T as TryFromValue<'a, S>>::try_from_value(self)
    }
}

impl<T: ValueSchema> Copy for Value<T> {}

impl<T: ValueSchema> Clone for Value<T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            _schema: PhantomData,
        }
    }
}

impl<T: ValueSchema> PartialEq for Value<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T: ValueSchema> Eq for Value<T> {}

impl<T: ValueSchema> Hash for Value<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T: ValueSchema> Ord for Value<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<T: ValueSchema> PartialOrd for Value<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: ValueSchema> Borrow<RawValue> for Value<S> {
    fn borrow(&self) -> &RawValue {
        &self.raw
    }
}

impl<T: ValueSchema> Debug for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Value<{}>({})",
            std::any::type_name::<T>(),
            ToHex::encode_hex::<String>(&self.raw.bytes)
        )
    }
}

pub trait ValueSchema: Sized + 'static {
    const ID: RawId;

    fn to_value<T: ToValue<Self>>(t: T) -> Value<Self> {
        t.to_value()
    }

    fn try_to_value<T: TryToValue<Self>>(
        t: T,
    ) -> Result<Value<Self>, <T as TryToValue<Self>>::Error> {
        t.try_to_value()
    }
}

pub trait ToValue<S: ValueSchema> {
    fn to_value(self) -> Value<S>;
}
pub trait FromValue<'a, S: ValueSchema> {
    fn from_value(v: &'a Value<S>) -> Self;
}

pub trait TryToValue<S: ValueSchema> {
    type Error;
    fn try_to_value(self) -> Result<Value<S>, Self::Error>;
}
pub trait TryFromValue<'a, S: ValueSchema>: Sized {
    type Error;
    fn try_from_value(v: &'a Value<S>) -> Result<Self, Self::Error>;
}

impl<S: ValueSchema> ToValue<S> for Value<S> {
    fn to_value(self) -> Value<S> {
        self
    }
}

impl<S: ValueSchema> ToValue<S> for &Value<S> {
    fn to_value(self) -> Value<S> {
        *self
    }
}

impl<'a, S: ValueSchema> FromValue<'a, S> for Value<S> {
    fn from_value(v: &'a Value<S>) -> Self {
        *v
    }
}

impl<'a, S: ValueSchema> FromValue<'a, S> for () {
    fn from_value(_v: &'a Value<S>) -> Self {
        ()
    }
}
