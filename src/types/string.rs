use bytes::Bytes;

use crate::{BlobParseError, Bloblike, Handle};

use super::Hash;

impl<'a> Bloblike<'a> for String {
    type Read = &'a str;

    fn from_blob(blob: &'a Bytes) -> Result<Self::Read, BlobParseError> {
        std::str::from_utf8(&blob[..])
            .map_err(|_| BlobParseError::new("failed to convert to utf-8 string"))
    }
    fn into_blob(self) -> Bytes {
        self.into()
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

    use crate::Handle;

    #[test]
    fn string_handle() {
        let s = String::from("hello world!");
        let h: Handle<Blake2b<U32>, String> = Handle::from(&s);
        let h2: Handle<Blake2b<U32>, String> = Handle::from(&s);

        assert!(h == h2);
    }
}
