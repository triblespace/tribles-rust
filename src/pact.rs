use std::mem;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU16;
use siphasher::sip128::{Hasher128, SipHasher24};
use crate::bitset::{ByteBitset};
use crate::bytetable::{ByteTable, ByteEntry};

#[repr(C)]
struct Branch<const N: usize, T: Clone> {
    leaf_count: u64,
    rc: AtomicU16,
    segment_count: u32, //TODO: increase this to a u48
    node_hash: u128,
    child_set: ByteBitset,
    children: ByteTable<N, Head<T>>,
}

#[repr(C)]
struct Infix<const N: usize, T: Clone> {
    child: Head<T>,
    rc: AtomicU16,
    infix: [u8; N],
}

//#[rustc_layout(debug)]
#[derive(Clone, Debug)]
#[repr(u8)]
enum Head<T: Clone> {
    None {padding: [u8; 15]} = 0,
    Branch1 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<1, T>>,
        phantom: PhantomData<Branch<1, T>>
        },
    Branch2 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<2, T>>,
        phantom: PhantomData<Branch<2, T>>
        },
    Branch4 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<4, T>>,
        phantom: PhantomData<Branch<4, T>>
        },
    Branch8 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<8, T>>,
        phantom: PhantomData<Branch<8, T>>
        },
    Branch16 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<16, T>>,
        phantom: PhantomData<Branch<16, T>>
        },
    Branch32 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<32, T>>,
        phantom: PhantomData<Branch<32, T>>
        },
    Branch64 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<64, T>>,
        phantom: PhantomData<Branch<64, T>>
        },
        /*
    Infix14 {},
    Infix30 {},
    Infix46 {},
    Infix62 {},

    */
    Leaf {
        infix: [u8; 8],
        start_depth: u8,
        value: T
    },
}


unsafe impl<T: Clone> ByteEntry for Head<T> {
    fn zeroed() -> Self {
        return Head::None {padding: unsafe {mem::zeroed()}};
    }

    fn key(&self) -> Option<u8> {
        match self {
            Head::None {..} => None,
            Head::Branch1 { infix: infix, ..} => Some(infix[0]),
            Head::Branch1 { infix: infix, ..} => Some(infix[0]),
            Head::Branch2 { infix: infix, ..} => Some(infix[0]),
            Head::Branch4 { infix: infix, ..} => Some(infix[0]),
            Head::Branch8 { infix: infix, ..} => Some(infix[0]),
            Head::Branch16 { infix: infix, ..} => Some(infix[0]),
            Head::Branch32 { infix: infix, ..} => Some(infix[0]),
            Head::Branch64 { infix: infix, ..} => Some(infix[0]),
            _ => None

        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head<()>>(), 16);
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable<1, Head<()>>>(), 64);
        assert_eq!(mem::size_of::<Branch<1, ()>>(), 64*2);
        assert_eq!(mem::size_of::<Branch<2, ()>>(), 64*3);
        assert_eq!(mem::size_of::<Branch<4, ()>>(), 64*5);
        assert_eq!(mem::size_of::<Branch<8, ()>>(), 64*9);
        assert_eq!(mem::size_of::<Branch<16, ()>>(), 64*17);
        assert_eq!(mem::size_of::<Branch<32, ()>>(), 64*33);
        assert_eq!(mem::size_of::<Branch<64, ()>>(), 64*65);
        assert_eq!(mem::size_of::<Branch<64, ()>>(), 64*65);
    }

    #[test]
    fn infix_size() {
        assert_eq!(mem::size_of::<Infix<14, ()>>(), 16*2);
        assert_eq!(mem::size_of::<Infix<30, ()>>(), 16*3);
        assert_eq!(mem::size_of::<Infix<46, ()>>(), 16*4);
        assert_eq!(mem::size_of::<Infix<62, ()>>(), 16*5);
    }
}