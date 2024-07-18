use std::marker::PhantomData;

use crate::types::Hash;

use crate::Value;

#[repr(transparent)]
pub struct Handle<H, T> {
    _digest: PhantomData<H>,
    _type: PhantomData<T>,
}

impl<H, T> From<Value<Handle<H, T>>> for Value<Hash<H>> {
    fn from(value: Value<Handle<H, T>>) -> Self {
        Value::new(value.bytes)
    }
}
