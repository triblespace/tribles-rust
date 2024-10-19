use crate::blob::BlobSchema;
use crate::id::RawId;
use crate::value::{
    schemas::hash::{Hash, HashProtocol},
    Value, ValueSchema,
};

use std::marker::PhantomData;

#[repr(transparent)]
pub struct Handle<H, T> {
    digest: Hash<H>,
    _type: PhantomData<T>,
}

impl<H: HashProtocol, T: BlobSchema> From<Value<Handle<H, T>>> for Value<Hash<H>> {
    fn from(value: Value<Handle<H, T>>) -> Self {
        Value::new(value.bytes)
    }
}

impl<H: HashProtocol, T: BlobSchema> ValueSchema for Handle<H, T> {
    const ID: RawId = H::SCHEMA_ID;
}
