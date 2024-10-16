use std::marker::PhantomData;

use crate::valueschemas::Hash;

use crate::{BlobSchema, RawId, Value, ValueSchema};

use super::HashProtocol;

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

impl<H: HashProtocol, T: BlobSchema> ValueSchema for Handle<H, T> {const ID: RawId = H::SCHEMA_ID;}
