use minibytes::Bytes;

use crate::{BlobParseError, Bloblike, Handle};

use super::Hash;

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

    fn read_blob(blob: Bytes) -> Result<Self, BlobParseError> {
        std::str::from_utf8(&blob[..])
            .map_err(|_| BlobParseError::new("failed to convert to utf-8 string"))?;
        Ok(ZCString(blob))
    }

    fn as_handle<H>(&self) -> Handle<H, Self>
    where
        H: digest::Digest + digest::OutputSizeUser<OutputSize = digest::consts::U32>,
    {
        let digest = H::digest(self.as_bytes());
        unsafe { Handle::new(Hash::new(digest.into())) }
    }
}

#[cfg(test)]
mod tests {
    use blake2::Blake2b;
    use digest::typenum::U32;

    use crate::{types::ZCString, Handle};

    #[test]
    fn string_handle() {
        let s: ZCString = String::from("hello world!").into();
        let h: Handle<Blake2b<U32>, ZCString> = Handle::from(&s);
        let h2: Handle<Blake2b<U32>, ZCString> = Handle::from(&s);

        assert!(h == h2);
    }
}
