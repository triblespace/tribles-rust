use crate::types::{Blob, BlobParseError, Bloblike};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct LongString(String);

impl LongString {
    pub fn new(s: String) -> LongString {
        LongString(s)
    }
}

impl From<LongString> for String {
    fn from(string: LongString) -> Self {
        string.0
    }
}

impl From<String> for LongString {
    fn from(string: String) -> Self {
        LongString(string)
    }
}

impl Bloblike for LongString {
    fn from_blob(blob: Blob) -> Result<Self, BlobParseError> {
        let s = String::from_utf8(blob.to_vec())
            .map_err(|_| BlobParseError::new(blob, "failed to convert to utf-8 string"))?;
        Ok(LongString(s))
    }
    fn into_blob(&self) -> Blob {
        let bytes = self.0.as_bytes();
        bytes.into()
    }
}

#[cfg(test)]
mod tests {
    use blake2::Blake2b;
    use digest::typenum::U32;

    use crate::types::handle::Handle;

    use super::LongString;

    #[test]
    fn handle() {
        let s = String::from("hello world!");
        let l: LongString = s.into();
        let h: Handle<Blake2b<U32>, LongString> = (&l).into();
        let h2: Handle<Blake2b<U32>, LongString> = (&l).into();

        assert!(h == h2);
    }
}
