use std::marker::PhantomData;

use crate::trible::{Value, Blob};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Handle<T>
where T: std::convert::From<Blob> {
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

