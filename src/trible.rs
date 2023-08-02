use std::convert::TryInto;

use crate::namespace::*;
use crate::pact::{KeyOrdering, KeySegmentation};
use arbitrary::Arbitrary;

pub const TRIBLE_LEN: usize = 64;
pub const E_START: usize = 0;
pub const E_END: usize = 15;
pub const A_START: usize = 16;
pub const A_END: usize = 31;
pub const V_START: usize = 32;
pub const V_END: usize = 63;

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

    pub fn raw(data: [u8; 64]) -> Trible {
        Self { data }
    }

    pub fn raw_values(e: Value, a: Value, v: Value) -> Option<Trible> {
        if e[16..32].iter().any(|&x| x != 0) || a[16..32].iter().any(|&x| x != 0) {
            return None;
        }

        let mut data = [0; TRIBLE_LEN];
        data[E_START..=E_END].copy_from_slice(&e[16..32]);
        data[A_START..=A_END].copy_from_slice(&a[16..32]);
        data[V_START..=V_END].copy_from_slice(&v[..]);

        Some(Self { data })
    }

    pub fn e(&self) -> Id {
        self.data[E_START..=E_END].try_into().unwrap()
    }
    pub fn a(&self) -> Id {
        self.data[A_START..=A_END].try_into().unwrap()
    }
    pub fn v(&self) -> Value {
        self.data[V_START..=V_END].try_into().unwrap()
    }

    pub fn e_as_value(&self) -> Value {
        let mut o = [0u8; 32];
        &mut o[16..=31].copy_from_slice(&self.data[E_START..=E_END]);
        o
    }
    pub fn a_as_value(&self) -> Value {
        let mut o = [0u8; 32];
        &mut o[16..=31].copy_from_slice(&self.data[A_START..=A_END]);
        o
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
            _ => panic!(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EAVOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for EAVOrder {
    fn tree_index(key_index: usize) -> usize {
        key_index
    }

    fn key_index(tree_index: usize) -> usize {
        tree_index
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EVAOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for EVAOrder {
    fn tree_index(key_index: usize) -> usize {
        match key_index {
            d @ E_START..=E_END => d,
            d @ A_START..=A_END => d + 32,
            d @ V_START..=V_END => d - 16,
            _ => panic!(),
        }
    }

    fn key_index(tree_index: usize) -> usize {
        match tree_index {
            d if d < 16 => d,
            d if d < 48 => d + 16,
            d => d - 32,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AEVOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for AEVOrder {
    fn tree_index(key_index: usize) -> usize {
        match key_index {
            d @ E_START..=E_END => d + 16,
            d @ A_START..=A_END => d - 16,
            d @ V_START..=V_END => d,
            _ => panic!(),
        }
    }

    fn key_index(tree_index: usize) -> usize {
        match tree_index {
            d if d < 16 => d + 16,
            d if d < 32 => d - 16,
            d => d,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AVEOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for AVEOrder {
    fn tree_index(key_index: usize) -> usize {
        match key_index {
            d @ E_START..=E_END => d + 48,
            d @ A_START..=A_END => d - 16,
            d @ V_START..=V_END => d - 16,
            _ => panic!(),
        }
    }

    fn key_index(tree_index: usize) -> usize {
        match tree_index {
            d if d < 16 => d + 16,
            d if d < 48 => d + 16,
            d => d - 48,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VEAOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for VEAOrder {
    fn tree_index(key_index: usize) -> usize {
        match key_index {
            d @ E_START..=E_END => d + 32,
            d @ A_START..=A_END => d + 32,
            d @ V_START..=V_END => d - 32,
            _ => panic!(),
        }
    }

    fn key_index(tree_index: usize) -> usize {
        match tree_index {
            d if d < 32 => d + 32,
            d if d < 48 => d - 32,
            d => d - 32,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VAEOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for VAEOrder {
    fn tree_index(key_index: usize) -> usize {
        match key_index {
            d @ E_START..=E_END => d + 48,
            d @ A_START..=A_END => d + 16,
            d @ V_START..=V_END => d - 32,
            _ => panic!(),
        }
    }

    fn key_index(tree_index: usize) -> usize {
        match tree_index {
            d if d < 32 => d + 32,
            d if d < 48 => d - 16,
            d => d - 48,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    #[test]
    fn order_eav() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        assert_eq!(EAVOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(EAVOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_eva() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ];
        assert_eq!(EVAOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(EVAOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_aev() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7,
            8, 9, 10, 11, 12, 13, 14, 15, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        assert_eq!(AEVOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(AEVOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_ave() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
            38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
            60, 61, 62, 63, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ];
        assert_eq!(AVEOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(AVEOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_vea() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ]; 
        let tree_bytes = [
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ];
        assert_eq!(VEAOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(VEAOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_vae() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ];
        assert_eq!(VAEOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(VAEOrder::key_ordered(&tree_bytes), key_bytes);
    }
}
