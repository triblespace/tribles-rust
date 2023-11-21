use std::marker::PhantomData;

use crate::trible::{Value, Blob};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Handle<T> {
    pub value: Value,
    _type: PhantomData<T>
}

impl<T> Handle<T>
where T: From<Blob> {
    pub fn new(value: Value) -> Handle<T> {
        Handle {
            value,
            _type: PhantomData
        }
    }
}

impl<T> From<Value> for Handle<T> {
    fn from(value: Value) -> Self {
        Handle {value, _type: PhantomData}
    }
}

impl<T> From<&Handle<T>> for Value {
    fn from(handle: &Handle<T>) -> Self {
        handle.value
    }
}
