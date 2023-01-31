use arbitrary::Arbitrary;

#[derive(Arbitrary, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Trible {
    pub data: [u8; 64],
}

impl Trible {
    pub fn new<E, A, V>(e: &E, a: &A, v: &V) -> Trible
    where
        E: Id,
        A: Id,
        V: Value,
    {
        let mut data = [0; 64];
        data[0..16].copy_from_slice(&mut Id::encode(e)[..]);
        data[16..32].copy_from_slice(&mut Id::encode(a)[..]);
        data[32..64].copy_from_slice(&mut Value::encode(v)[..]);

        Self { data }
    }
    /*
        pub fn ordered(&self) -> [u8; 64] {

        }
    */
}

pub trait Id {
    fn decode(data: [u8; 16]) -> Self;
    fn encode(id: &Self) -> [u8; 16];
}

pub trait Value {
    fn decode(data: [u8; 32]) -> Self;
    fn encode(value: &Self) -> [u8; 32];
}
