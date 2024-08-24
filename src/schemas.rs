//! This is a collection of Rust types that can be (de)serialized as
//! [Value]s, and [Blob]s.

pub mod ed25519;
pub mod f256;
pub mod fr256;
pub mod genid;
pub mod handle;
pub mod hash;
pub mod iu256;
pub mod shortstring;
pub mod time;
pub mod zc;
pub mod zcstring;

use std::{borrow::Borrow, fmt::Debug};

pub use genid::*;
pub use handle::*;
pub use hash::*;
pub use shortstring::*;
pub use time::*;
pub use zcstring::*;

use crate::Value;

//use crate::Value;

pub trait ValueSchema: Sized {
    fn pack<T: Pack<Self>  + ?Sized>(t: &T) -> Value<Self> {
        t.borrow().pack()
    }

    fn try_pack<T: TryPack<Self> + ?Sized>(t: &T) -> Result<Value<Self>, <T as TryPack<Self>>::Error> {
        t.borrow().try_pack()
    }
}

pub trait Pack<S: ValueSchema> {
    fn pack(&self) -> Value<S>;
}
pub trait Unpack<'a, S: ValueSchema> {
    fn unpack(v: &'a Value<S>) -> Self;
}

pub trait TryPack<S: ValueSchema> {
    type Error;
    fn try_pack(&self) -> Result<Value<S>, Self::Error>;
}
pub trait TryUnpack<'a, S: ValueSchema>: Sized {
    type Error;
    fn try_unpack(v: &'a Value<S>) -> Result<Self, Self::Error>;
}

pub struct UnknownValue {}
impl ValueSchema for UnknownValue {}
