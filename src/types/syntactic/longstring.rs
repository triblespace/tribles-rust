use blake2::{digest::typenum::U32, Blake2b, Digest};

use crate::{
    handle_value,
    trible::{Blob, Value},
    types::handle::Handle,
};

#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LongString(String);

handle_value!(LongString);

impl LongString {
    pub fn new(s: String) -> LongString {
        LongString(s)
    }
}

impl From<&LongString> for String {
    fn from(string: &LongString) -> Self {
        string.0.clone()
    }
}

impl From<String> for LongString {
    fn from(string: String) -> Self {
        LongString(string)
    }
}

impl From<&LongString> for Blob {
    fn from(string: &LongString) -> Self {
        let bytes = string.0.as_bytes();
        bytes.into()
    }
}

impl From<Blob> for LongString {
    fn from(blob: Blob) -> Self {
        LongString(String::from_utf8(blob.to_vec()).expect("failed to decode LongString"))
    }
}

impl From<&Blob> for Handle<LongString> {
    fn from(blob: &Blob) -> Self {
        Handle::new(Blake2b::<U32>::digest(blob).into())
    }
}
