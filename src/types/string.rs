use bytes::Bytes;

use crate::{Blob, BlobParseError, Bloblike};

impl Bloblike for String {
    fn from_blob(blob: Blob) -> Result<Self, BlobParseError> {
        let s = String::from_utf8(blob.to_vec())
            .map_err(|_| BlobParseError::new(blob, "failed to convert to utf-8 string"))?;
        Ok(s)
    }
    fn into_blob(&self) -> Blob {
        Bytes::copy_from_slice(self.as_bytes().into())
    }
}

#[cfg(test)]
mod tests {
    use blake2::Blake2b;
    use digest::typenum::U32;

    use crate::types::handle::Handle;

    #[test]
    fn handle() {
        let s = String::from("hello world!");
        let h: Handle<Blake2b<U32>, String> = (&s).into();
        let h2: Handle<Blake2b<U32>, String> = (&s).into();

        assert!(h == h2);
    }
}
