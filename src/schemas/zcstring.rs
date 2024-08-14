use anybytes::Bytes;
use digest::{Digest, typenum::U32};
use crate::{BlobParseError, Bloblike, Handle, Value};

pub struct ZCString(Bytes);

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

impl Bloblike for ZCString {
    fn into_blob(self) -> Bytes {
        self.0
    }

    fn from_blob(blob: Bytes) -> Result<Self, BlobParseError> {
        std::str::from_utf8(&blob[..])
            .map_err(|_| BlobParseError::new("failed to convert to utf-8 string"))?;
        Ok(ZCString(blob))
    }

    fn as_handle<H>(&self) -> Value<Handle<H, Self>>
    where
        H: Digest<OutputSize = U32>,
    {
        let digest = H::digest(self.as_bytes());
        Value::new(digest.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{schemas::{hash::Blake2b, ZCString}, Bloblike, Handle, Value};

    #[test]
    fn string_handle() {
        let s: ZCString = String::from("hello world!").into();
        let h: Value<Handle<Blake2b, ZCString>> = (&s).as_handle();
        let h2: Value<Handle<Blake2b, ZCString>> = (&s).as_handle();

        assert!(h == h2);
    }
}
