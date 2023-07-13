use blake2::{Digest, Blake2b, digest::typenum::U32};

use crate::namespace::Value;

#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LongString {
    inner: String,
}

impl LongString {
    pub fn new(s: String) -> LongString {
        LongString { inner: s }
    }
}

impl From<&LongString> for String {
    fn from(string: &LongString) -> Self {
        string.inner.clone()
    }
}

impl From<String> for LongString {
    fn from(string: String) -> Self {
        LongString {
            inner: string,
        }
    }
}

impl From<&LongString> for Value {
    fn from(string: &LongString) -> Self {
        let bytes = string.inner.as_bytes();
        let data: [u8; 32] = Blake2b::<U32>::digest(&bytes).into();
        data
    }
}
