//! This is a collection of Rust types that can be (de)serialized as
//! [Value]s, and [Blob]s.

pub mod handle;
pub mod ed25519;
pub mod f256;
pub mod fr256;
pub mod iu256;
pub mod hash;
pub mod shortstring;
pub mod time;
pub mod zcstring;
pub mod zc;

use std::borrow::Borrow;

pub use hash::Hash;
pub use handle::Handle;
pub use shortstring::*;
pub use time::*;
pub use zcstring::*;

use crate::Value;

//use crate::Value;

pub trait Schema: Sized {
    fn pack<T: Pack<Self>, B: Borrow<T>>(t: B) -> Value<Self> {
        t.borrow().pack()
    }
}

pub trait Pack<S: Schema> {
    fn pack(&self) -> Value<S>;
}
pub trait Unpack<'a, S: Schema> {
    fn unpack(v: &'a Value<S>) -> Self;
}

pub trait TryPack<S: Schema> {
    type Error;
    fn try_pack(&self) -> Result<Value<S>, Self::Error>;
}
pub trait TryUnpack<'a, S: Schema>: Sized {
    type Error;
    fn try_unpack(v: &'a Value<S>) -> Result<Self, Self::Error>;
}

pub struct Unknown {}
impl Schema for Unknown {}