//! This is a collection of Rust types that can be (de)serialized as [crate::prelude::Value]s.

pub mod ed25519;
pub mod f256;
pub mod genid;
pub mod hash;
pub mod iu256;
pub mod r256;
pub mod shortstring;
pub mod time;

use crate::id::Id;
use crate::id_hex;
use crate::value::ValueSchema;

pub struct UnknownValue {}
impl ValueSchema for UnknownValue {
    const VALUE_SCHEMA_ID: Id = id_hex!("4EC697E8599AC79D667C722E2C8BEBF4");
}
