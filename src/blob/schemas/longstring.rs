use crate::blob::{Blob, BlobSchema, FromBlob, ToBlob, TryFromBlob};
use crate::id::Id;
use crate::id_hex;

use std::str::Utf8Error;

use anybytes::{view::ViewError, View};

pub struct LongString {}

impl BlobSchema for LongString {
    const BLOB_SCHEMA_ID: Id = id_hex!("8B173C65B7DB601A11E8A190BD774A79");
}

impl TryFromBlob<'_, LongString> for View<str> {
    type Error = ViewError;

    fn try_from_blob(b: &Blob<LongString>) -> Result<Self, Self::Error> {
        (&b.bytes).clone().view()
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

impl ToBlob<LongString> for View<str> {
    fn to_blob(self) -> Blob<LongString> {
        Blob::new(self.bytes())
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
    use anybytes::{Bytes, View};

    use crate::{
        blob::{schemas::longstring::LongString, ToBlob},
        value::{
            schemas::hash::{Blake3, Handle},
            Value,
        },
    };

    #[test]
    fn string_handle() {
        let s: View<str> = Bytes::from(String::from("hello world!")).view().unwrap();
        let h: Value<Handle<Blake3, LongString>> = s.clone().to_blob().get_handle();
        let h2: Value<Handle<Blake3, LongString>> = s.clone().to_blob().get_handle();

        assert!(h == h2);
    }
}
