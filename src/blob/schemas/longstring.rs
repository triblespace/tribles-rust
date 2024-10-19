use crate::blob::{Blob, BlobSchema, PackBlob, TryUnpackBlob};
use crate::id::RawId;

use std::{convert::TryInto, str::Utf8Error};

use anybytes::PackedStr;
use hex_literal::hex;

pub struct LongString {}

impl BlobSchema for LongString {
    const ID: RawId = hex!("8B173C65B7DB601A11E8A190BD774A79");
}

impl TryUnpackBlob<'_, LongString> for PackedStr {
    type Error = Utf8Error;

    fn try_unpack(b: &Blob<LongString>) -> Result<Self, Self::Error> {
        (&b.bytes).try_into()
    }
}

impl PackBlob<LongString> for PackedStr {
    fn pack(&self) -> Blob<LongString> {
        Blob::new(self.bytes())
    }
}

#[cfg(test)]
mod tests {
    use anybytes::PackedStr;

    use crate::{
        blob::{PackBlob, schemas::longstring::LongString},
        value::{Value, schemas::{hash::Blake3, handle::Handle}},
    };

    #[test]
    fn string_handle() {
        let s: PackedStr = String::from("hello world!").into();
        let h: Value<Handle<Blake3, LongString>> = s.pack().as_handle();
        let h2: Value<Handle<Blake3, LongString>> = s.pack().as_handle();

        assert!(h == h2);
    }
}
