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

pub use genid::*;
pub use handle::*;
pub use hash::*;
pub use shortstring::*;
pub use time::*;

use crate::Value;

pub trait ValueSchema: Sized {
    fn pack<T: PackValue<Self>  + ?Sized>(t: &T) -> Value<Self> {
        t.pack()
    }

    fn try_pack<T: TryPackValue<Self> + ?Sized>(t: &T) -> Result<Value<Self>, <T as TryPackValue<Self>>::Error> {
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

pub struct UnknownValue {}
impl ValueSchema for UnknownValue {}
