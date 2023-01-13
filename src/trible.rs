use arbitrary::Arbitrary;

#[derive(Arbitrary, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Trible {
    pub data: [u8; 64],
}

/*
impl Trible {
    fn toEAV(&self) -> [u8; 64] {
    }
}
*/
