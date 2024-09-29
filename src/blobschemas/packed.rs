use std::{convert::TryInto, str::Utf8Error};

use anybytes::{packed::PackError, Packed, PackedSlice, PackedStr};
use zerocopy::FromBytes;

use crate::Blob;

use super::{BlobSchema, PackBlob, TryUnpackBlob};

impl<T> BlobSchema for Packed<T> {}

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

impl BlobSchema for PackedStr {}

impl TryUnpackBlob<'_, PackedStr> for PackedStr {
    type Error = Utf8Error;

    fn try_unpack(b: &Blob<PackedStr>) -> Result<Self, Self::Error> {
        (&b.bytes).try_into()
    }
}

impl PackBlob<PackedStr> for PackedStr {
    fn pack(&self) -> Blob<PackedStr> {
        Blob::new(self.bytes())
    }
}

#[cfg(test)]
mod tests {
    use anybytes::PackedStr;

    use crate::{
        blobschemas::PackBlob,
        valueschemas::{hash::Blake2b, Handle},
        Value,
    };

    #[test]
    fn string_handle() {
        let s: PackedStr = String::from("hello world!").into();
        let h: Value<Handle<Blake2b, PackedStr>> = s.pack().as_handle();
        let h2: Value<Handle<Blake2b, PackedStr>> = s.pack().as_handle();

        assert!(h == h2);
    }
}
