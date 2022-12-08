use std::mem;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU16;
use ux::u48;
use siphasher::sip128::{Hasher128, SipHasher24};
use crate::bitset::{ByteBitset};
use crate::bytetable::{ByteTable, ByteEntry};

#[repr(C)]
struct Branch<const N: usize> {
    leaf_count: u64,
    rc: AtomicU16,
    segment_count: u48,
    node_hash: u128,
    child_set: ByteBitset,
    children: ByteTable<N, Head>,
}

#[derive(Clone, Debug)]
#[repr(C, u8)]
enum Head {
    None {padding: [u8; 15]} = 0,
    Branch1 {infix: [u8; 5],
             start_depth: u8,
             branch_depth: u8,
             ptr: NonNull<Branch<1>>,
             phantom: PhantomData<Branch<1>>,
             },
             /*
    Branch2 {},
    Branch4 {},
    Branch8 {},
    Branch16 {},
    Branch32 {},
    Branch64 {},
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
            Head::Branch1 { infix: infix, ..} => Some(infix[0])
        }
    }
}