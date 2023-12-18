use blake2::{digest::typenum::U32, Blake2b };

use crate::{
    handle_value,
    trible::Blob,
};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct LongString(String);

handle_value!(Blake2b::<U32>, LongString);

impl LongString {
    pub fn new(s: String) -> LongString {
        LongString(s)
    }
}

impl From<&LongString> for String {
    fn from(string: &LongString) -> Self {
        string.0.clone()
    }
}

impl From<String> for LongString {
    fn from(string: String) -> Self {
        LongString(string)
    }
}

impl From<&LongString> for Blob {
    fn from(string: &LongString) -> Self {
        let bytes = string.0.as_bytes();
        bytes.into()
    }
}

impl From<Blob> for LongString {
    fn from(blob: Blob) -> Self {
        LongString(String::from_utf8(blob.to_vec()).expect("failed to decode LongString"))
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
        let h: Handle<Blake2b::<U32>, LongString> = (&l).into();
        let h2: Handle<Blake2b::<U32>, LongString> = (&l).into();

        assert!(h == h2);
    }
}
