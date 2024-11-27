use crate::blob::{Blob, BlobSchema, FromBlob, ToBlob, TryFromBlob};
use crate::id::RawId;

use std::{convert::TryInto, str::Utf8Error};

use anybytes::PackedStr;
use hex_literal::hex;

pub struct LongString {}

impl BlobSchema for LongString {
    const ID: RawId = RawId::new(&hex!("8B173C65B7DB601A11E8A190BD774A79"));
}

impl TryFromBlob<'_, LongString> for PackedStr {
    type Error = Utf8Error;

    fn try_from_blob(b: &Blob<LongString>) -> Result<Self, Self::Error> {
        (&b.bytes).try_into()
    }
}

impl<'a> TryFromBlob<'a, LongString> for &'a str {
    type Error = Utf8Error;

    fn try_from_blob(b: &'a Blob<LongString>) -> Result<Self, Self::Error> {
        std::str::from_utf8(&b.bytes[..])
    }
}

impl<'a> FromBlob<'a, LongString> for &'a str {
    fn from_blob(b: &'a Blob<LongString>) -> Self {
        std::str::from_utf8(&b.bytes[..]).unwrap()
    }
}

impl ToBlob<LongString> for PackedStr {
    fn to_blob(self) -> Blob<LongString> {
        Blob::new(self.unwrap())
    }
}

impl ToBlob<LongString> for &'static str {
    fn to_blob(self) -> Blob<LongString> {
        Blob::new(self.into())
    }
}

impl ToBlob<LongString> for String {
    fn to_blob(self) -> Blob<LongString> {
        Blob::new(self.into())
    }
}

#[cfg(test)]
mod tests {
    use anybytes::PackedStr;

    use crate::{
        blob::{schemas::longstring::LongString, ToBlob},
        value::{
            schemas::hash::{Blake3, Handle},
            Value,
        },
    };

    #[test]
    fn string_handle() {
        let s: PackedStr = String::from("hello world!").into();
        let h: Value<Handle<Blake3, LongString>> = s.clone().to_blob().as_handle();
        let h2: Value<Handle<Blake3, LongString>> = s.clone().to_blob().as_handle();

        assert!(h == h2);
    }
}
