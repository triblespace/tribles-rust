use blake2::{digest::typenum::U32, Blake2b, Digest};

use crate::{trible::{Value, Blob}, hash_value};

#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LongString(String);

hash_value!(LongString);

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

impl From<&LongString> for Value {
    fn from(string: &LongString) -> Self {
        let bytes = string.0.as_bytes();
        let data: [u8; 32] = Blake2b::<U32>::digest(&bytes).into();
        data
    }
}

impl From<&LongString> for (Value, Option<Blob>) {
    fn from(string: &LongString) -> Self {
        let bytes = string.0.as_bytes();
        let data: [u8; 32] = Blake2b::<U32>::digest(&bytes).into();
        (data, Some(bytes.into()))
    }
}

impl From<Blob> for LongString {
    fn from(blob: Blob) -> Self {
        LongString(String::from_utf8(blob.to_vec()).expect("failed to decode LongString"))
    }
}
