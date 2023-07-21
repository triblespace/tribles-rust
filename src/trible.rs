use crate::namespace::*;
use crate::pact::{KeyOrdering, KeySegmentation};
use arbitrary::Arbitrary;

const TRIBLE_LEN: usize = 64;
const E_START: usize = 0;
const E_END: usize = 15;
const A_START: usize = 16;
const A_END: usize = 15;
const V_START: usize = 32;
const V_END: usize = 63;

#[derive(Arbitrary, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Trible {
    pub data: [u8; TRIBLE_LEN],
}

impl Trible {
    pub fn new<E, A, V>(e: E, a: A, v: V) -> Trible
    where
        for<'a> Id: From<&'a E> + From<&'a A>,
        for<'a> Value: From<&'a V>,
    {
        let mut data = [0; TRIBLE_LEN];
        data[E_START..=E_END].copy_from_slice(&mut Id::from(&e)[..]);
        data[A_START..=A_END].copy_from_slice(&mut Id::from(&a)[..]);
        data[V_START..=V_END].copy_from_slice(&mut Value::from(&v)[..]);

        Self { data }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TribleSegmentation {}

impl KeySegmentation<64> for TribleSegmentation {
    fn segment(depth: usize) -> usize {
        match depth {
            E_START..=E_END => 0,
            A_START..=A_END => 1,
            V_START..=V_END => 2,
            _ => panic!()
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EAVOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for EAVOrder {
    fn reorder(depth: usize) -> usize {
        depth
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EVAOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for EVAOrder {
    fn reorder(depth: usize) -> usize {
        match depth {
            d if d < 16 => d,
            d if d < 48 => d + 16,
            d => d - 32,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AEVOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for AEVOrder {
    fn reorder(depth: usize) -> usize {
        match depth {
            d if d < 16 => d + 16,
            d if d < 32 => d - 16,
            d => d,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AVEOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for AVEOrder {
    fn reorder(depth: usize) -> usize {
        match depth {
            d if d < 16 => d + 16,
            d if d < 48 => d + 16,
            d => d - 48,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VEAOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for VEAOrder {
    fn reorder(depth: usize) -> usize {
        match depth {
            d if d < 32 => d + 32,
            d if d < 48 => d - 32,
            d => d - 32,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VAEOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for VAEOrder {
    fn reorder(depth: usize) -> usize {
        match depth {
            d if d < 32 => d + 32,
            d if d < 48 => d - 16,
            d => d - 48,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pact::reordered;

    use super::*;

    #[rustfmt::skip]
    #[test]
    fn order_eav() {
        let canonical_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let reordered_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        assert_eq!(reordered::<64, EAVOrder>(&canonical_bytes), reordered_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_eva() {
        let canonical_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let reordered_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ];
        assert_eq!(reordered::<64, EVAOrder>(&canonical_bytes), reordered_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_aev() {
        let canonical_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let reordered_bytes = [
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7,
            8, 9, 10, 11, 12, 13, 14, 15, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        assert_eq!(reordered::<64, AEVOrder>(&canonical_bytes), reordered_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_ave() {
        let canonical_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let reordered_bytes = [
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
            38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
            60, 61, 62, 63, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ];
        assert_eq!(reordered::<64, AVEOrder>(&canonical_bytes), reordered_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_vea() {
        let canonical_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ]; 
        let reordered_bytes = [
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ];
        assert_eq!(reordered::<64, VEAOrder>(&canonical_bytes), reordered_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_vae() {
        let canonical_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let reordered_bytes = [
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ];
        assert_eq!(reordered::<64, VAEOrder>(&canonical_bytes), reordered_bytes);
    }
}
