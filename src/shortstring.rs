#[derive(Arbitrary, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct ShortString {
    data: [u8; 32],
}

impl Value for ShortString {
    fn decode(data: [u8; 32], blob: fn() -> Option<Vec<u8>>) -> Self {
        ShortString { data }
    }
    fn encode(value: Self) -> ([u8; 32], Option<Vec<u8>>) {
        (value.data, None)
    }
}

