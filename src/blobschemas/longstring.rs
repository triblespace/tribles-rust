use std::{convert::TryInto, str::Utf8Error};

use anybytes::PackedStr;
use hex_literal::hex;

use crate::{Blob, RawId};

use super::{BlobSchema, PackBlob, TryUnpackBlob};

/*
impl<T> BlobSchema for Packed<T> {
    const ID: RawId = TypeId::of::<Packed<T>>();
}

impl<T> PackBlob<Packed<T>> for Packed<T> {
    fn pack(&self) -> crate::Blob<Packed<T>> {
        Blob::new(self.bytes())
    }
}

impl<'a, T> TryUnpackBlob<'a, Packed<T>> for Packed<T>
where
    T: FromBytes,
{
    type Error = PackError;

    fn try_unpack(b: &'a Blob<Self>) -> Result<Self, Self::Error> {
        b.bytes.clone().try_into()
    }
}

impl<T> BlobSchema for PackedSlice<T> {}

impl<T> PackBlob<PackedSlice<T>> for PackedSlice<T> {
    fn pack(&self) -> crate::Blob<PackedSlice<T>> {
        Blob::new(self.bytes())
    }
}

impl<'a, T> TryUnpackBlob<'a, PackedSlice<T>> for PackedSlice<T>
where
    T: FromBytes,
{
    type Error = PackError;

    fn try_unpack(b: &'a Blob<Self>) -> Result<Self, Self::Error> {
        b.bytes.clone().try_into()
    }
}
*/

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
        blobschemas::{longstring::LongString, PackBlob},
        valueschemas::{hash::Blake3, Handle},
        Value,
    };

    #[test]
    fn string_handle() {
        let s: PackedStr = String::from("hello world!").into();
        let h: Value<Handle<Blake3, LongString>> = s.pack().as_handle();
        let h2: Value<Handle<Blake3, LongString>> = s.pack().as_handle();

        assert!(h == h2);
    }
}
