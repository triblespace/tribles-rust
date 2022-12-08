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
struct Branch<const N: usize> {
    leaf_count: u64,
    rc: AtomicU16,
    segment_count: u32, //TODO: increase this to a u48
    node_hash: u128,
    child_set: ByteBitset,
    children: ByteTable<N, Head>,
}

//#[rustc_layout(debug)]
#[derive(Clone, Debug)]
#[repr(u8)]
enum Head {
    None {padding: [u8; 15]} = 0,
    Branch1 {
        start_depth: u8,
        branch_depth: u8,
        infix: [u8; 5],
        ptr: NonNull<Branch<1>>,
        phantom: PhantomData<Branch<1>>
        },
    Branch2 {
        start_depth: u8,
        branch_depth: u8,
        infix: [u8; 5],
        ptr: NonNull<Branch<2>>,
        phantom: PhantomData<Branch<2>>
        },
    Branch4 {
        start_depth: u8,
        branch_depth: u8,
        infix: [u8; 5],
        ptr: NonNull<Branch<4>>,
        phantom: PhantomData<Branch<4>>
        },
    Branch8 {
        start_depth: u8,
        branch_depth: u8,
        infix: [u8; 5],
        ptr: NonNull<Branch<8>>,
        phantom: PhantomData<Branch<8>>
        },
    Branch16 {
        start_depth: u8,
        branch_depth: u8,
        infix: [u8; 5],
        ptr: NonNull<Branch<16>>,
        phantom: PhantomData<Branch<16>>
        },
    Branch32 {
        start_depth: u8,
        branch_depth: u8,
        infix: [u8; 5],
        ptr: NonNull<Branch<32>>,
        phantom: PhantomData<Branch<32>>
        },
    Branch64 {
        start_depth: u8,
        branch_depth: u8,
        infix: [u8; 5],
        ptr: NonNull<Branch<64>>,
        phantom: PhantomData<Branch<64>>
        },
        /*
    Infix2: {},
    Infix3: {},
    Infix4: {},
    Leaf: {},
    */
}


unsafe impl ByteEntry for Head {
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


        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head>(), 16);
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable<1, Head>>(), 64);
        assert_eq!(mem::size_of::<Branch<1>>(), 64*2);
        assert_eq!(mem::size_of::<Branch<2>>(), 64*3);
        assert_eq!(mem::size_of::<Branch<4>>(), 64*5);
        assert_eq!(mem::size_of::<Branch<8>>(), 64*9);
        assert_eq!(mem::size_of::<Branch<16>>(), 64*17);
        assert_eq!(mem::size_of::<Branch<32>>(), 64*33);
        assert_eq!(mem::size_of::<Branch<64>>(), 64*65);
    }
}