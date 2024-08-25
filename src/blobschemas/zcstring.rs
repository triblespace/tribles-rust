use std::str::Utf8Error;

use crate::{blobschemas::BlobSchema, Blob};
use anybytes::Bytes;

use super::{PackBlob, TryUnpackBlob};

pub struct ZCString(Bytes);

impl BlobSchema for ZCString {}

impl TryUnpackBlob<'_, ZCString> for ZCString {
    type Error = Utf8Error;

    fn try_unpack(b: &Blob<ZCString>) -> Result<Self, Self::Error> {
        std::str::from_utf8(&b.bytes[..])?;
        Ok(ZCString(b.bytes.clone()))
    }
}

impl PackBlob<ZCString> for ZCString {
    fn pack(&self) -> Blob<ZCString> {
        Blob::new(self.0.clone())
    }
}

impl<'a> TryUnpackBlob<'a, ZCString> for &'a str {
    type Error = Utf8Error;

    fn try_unpack(b: &'a Blob<ZCString>) -> Result<Self, Self::Error> {
        std::str::from_utf8(&b.bytes[..])
    }
}

impl std::ops::Deref for ZCString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl From<String> for ZCString {
    fn from(value: String) -> Self {
        ZCString(value.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        blobschemas::{PackBlob, ZCString}, valueschemas::{hash::Blake2b, Handle}, Value
    };

    #[test]
    fn string_handle() {
        let s: ZCString = String::from("hello world!").into();
        let h: Value<Handle<Blake2b, ZCString>> = s.pack().as_handle();
        let h2: Value<Handle<Blake2b, ZCString>> = s.pack().as_handle();

        assert!(h == h2);
    }
}
