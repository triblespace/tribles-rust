use arbitrary::Arbitrary;
use crate::pact::KeyProperties;

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

    pub fn order_eav(&self) -> [u8; 64] {
        self.data
    }

    pub fn order_aev(&self) -> [u8; 64] {
        let mut data = [0; 64];
        data[16..32].copy_from_slice(&self.data[0..16]);
        data[0..16].copy_from_slice(&self.data[16..32]);
        data[32..64].copy_from_slice(&self.data[32..64]);
        data
    }

    pub fn order_ave(&self) -> [u8; 64] {
        let mut data = [0; 64];
        data[48..64].copy_from_slice(&self.data[0..16]);
        data[0..16].copy_from_slice(&self.data[16..32]);
        data[16..48].copy_from_slice(&self.data[32..64]);
        data
    }
}

pub trait Id {
    fn decode(data: [u8; 16]) -> Self;
    fn encode(id: &Self) -> [u8; 16];
}

pub trait Value {
    fn decode(data: [u8; 32]) -> Self;
    fn encode(value: &Self) -> [u8; 32];
}

#[derive(Copy, Clone, Debug)]
pub struct EAVOrder {}

impl<const KEY_LEN: usize> KeyProperties<KEY_LEN> for EAVOrder {
    fn reorder(depth: usize) -> usize {
        depth
    }
    fn segment(depth: usize) -> usize {
        match depth {
            d if d < 16 => 0,
            d if d < 32 => 1,
            _ => 2,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AEVOrder {}

impl<const KEY_LEN: usize> KeyProperties<KEY_LEN> for AEVOrder {
    fn reorder(depth: usize) -> usize {
        match depth {
            d if d < 16 => d + 16,
            d if d < 32 => d - 16,
            d => d,
        }
    }
    fn segment(depth: usize) -> usize {
        match depth {
            d if d < 16 => 1,
            d if d < 32 => 0,
            _ => 2,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AVEOrder {}

impl<const KEY_LEN: usize> KeyProperties<KEY_LEN> for AVEOrder {
    fn reorder(depth: usize) -> usize {
        match depth {
            d if d < 16 => d + 16,
            d if d < 48 => d + 16,
            d => d - 48,
        }
    }
    fn segment(depth: usize) -> usize {
        match depth {
            d if d < 16 => 1,
            d if d < 48 => 2,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pact::reordered;

    use super::*;

    #[test]
    fn order_eav() {
        let canonical_bytes =
           [ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15,
            16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
            32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,
            48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63];
        let reordered_bytes =
            [ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15,
             16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
             32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,
             48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63];
        assert_eq!(reordered::<64, EAVOrder>(&canonical_bytes), reordered_bytes);
    }

    #[test]
    fn order_aev() {
        let canonical_bytes =
           [ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15,
            16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
            32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,
            48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63];
        let reordered_bytes =
            [16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
             0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15,
             32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,
             48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63];
        assert_eq!(reordered::<64, AEVOrder>(&canonical_bytes), reordered_bytes);
    }

    #[test]
    fn order_ave() {
        let canonical_bytes =
           [ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15,
            16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
            32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,
            48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63];
        let reordered_bytes =
            [16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
             32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,
             48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,
             0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15];
        assert_eq!(reordered::<64, AVEOrder>(&canonical_bytes), reordered_bytes);
    }
}
