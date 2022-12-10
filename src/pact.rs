use crate::bitset::ByteBitset;
use crate::bytetable::{ByteEntry, ByteTable};
use siphasher::sip128::{Hasher128, SipHasher24};
use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU16;

trait SizeLimited<const N: usize>: Sized {
    const UNUSED: usize = N - std::mem::size_of::<Self>();
}

impl<A: Sized, const N: usize> SizeLimited<N> for A {}

#[repr(C)]
struct Branch<const N: usize, T: SizeLimited<13> + Clone>
where
    [u8; <T as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    leaf_count: u64,
    rc: AtomicU16,
    segment_count: u32, //TODO: increase this to a u48
    node_hash: u128,
    child_set: ByteBitset,
    children: ByteTable<N, Head<T>>,
}

#[repr(C)]
struct Infix<const N: usize, T: SizeLimited<13> + Clone>
where
    [u8; <T as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    child: Head<T>,
    rc: AtomicU16,
    infix: [u8; N],
}

//#[rustc_layout(debug)]
#[derive(Clone, Debug)]
#[repr(u8)]
enum Head<T: SizeLimited<13> + Clone>
where
    [u8; <T as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    None {
        padding: [u8; 15],
    } = 0,
    Branch1 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<1, T>>,
        phantom: PhantomData<Branch<1, T>>,
    },
    Branch2 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<2, T>>,
        phantom: PhantomData<Branch<2, T>>,
    },
    Branch4 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<4, T>>,
        phantom: PhantomData<Branch<4, T>>,
    },
    Branch8 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<8, T>>,
        phantom: PhantomData<Branch<8, T>>,
    },
    Branch16 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<16, T>>,
        phantom: PhantomData<Branch<16, T>>,
    },
    Branch32 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<32, T>>,
        phantom: PhantomData<Branch<32, T>>,
    },
    Branch64 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<64, T>>,
        phantom: PhantomData<Branch<64, T>>,
    },
    Infix14 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Infix<14, T>>,
        phantom: PhantomData<Infix<14, T>>,
    },
    Infix30 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Infix<30, T>>,
        phantom: PhantomData<Infix<30, T>>,
    },
    Infix46 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Infix<46, T>>,
        phantom: PhantomData<Infix<46, T>>,
    },
    Infix62 {
        infix: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Infix<62, T>>,
        phantom: PhantomData<Infix<62, T>>,
    },
    Leaf {
        infix: [u8; <T as SizeLimited<13>>::UNUSED + 1],
        start_depth: u8,
        value: T,
    },
}

unsafe impl<T: SizeLimited<13> + Clone> ByteEntry for Head<T>
where
    [u8; <T as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn zeroed() -> Self {
        return Head::None {
            padding: unsafe { mem::zeroed() },
        };
    }

    fn key(&self) -> Option<u8> {
        match self {
            Head::None { .. } => None,
            Head::Branch1 { infix: infix, .. } => Some(infix[0]),
            Head::Branch1 { infix: infix, .. } => Some(infix[0]),
            Head::Branch2 { infix: infix, .. } => Some(infix[0]),
            Head::Branch4 { infix: infix, .. } => Some(infix[0]),
            Head::Branch8 { infix: infix, .. } => Some(infix[0]),
            Head::Branch16 { infix: infix, .. } => Some(infix[0]),
            Head::Branch32 { infix: infix, .. } => Some(infix[0]),
            Head::Branch64 { infix: infix, .. } => Some(infix[0]),
            Head::Infix14 { infix: infix, .. } => Some(infix[0]),
            Head::Infix30 { infix: infix, .. } => Some(infix[0]),
            Head::Infix46 { infix: infix, .. } => Some(infix[0]),
            Head::Infix62 { infix: infix, .. } => Some(infix[0]),
            Head::Leaf { infix: infix, .. } => Some(infix[0]),
            _ => None,
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
        assert_eq!(mem::size_of::<Head<u64>>(), 16);
    }

    #[test]
    fn leaf_infix_size() {
        let head_twig = Head::<()>::Leaf {
            infix: unsafe { mem::zeroed() },
            start_depth: 0,
            value: (),
        };
        if let Head::<()>::Leaf { infix, .. } = head_twig {
            assert_eq!(infix.len(), 14);
        }

        let head = Head::<u64>::Leaf {
            infix: unsafe { mem::zeroed() },
            start_depth: 0,
            value: 0,
        };
        if let Head::<u64>::Leaf { infix, .. } = head {
            assert_eq!(infix.len(), 6);
        }
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable<1, Head<()>>>(), 64);
        assert_eq!(mem::size_of::<Branch<1, ()>>(), 64 * 2);
        assert_eq!(mem::size_of::<Branch<2, ()>>(), 64 * 3);
        assert_eq!(mem::size_of::<Branch<4, ()>>(), 64 * 5);
        assert_eq!(mem::size_of::<Branch<8, ()>>(), 64 * 9);
        assert_eq!(mem::size_of::<Branch<16, ()>>(), 64 * 17);
        assert_eq!(mem::size_of::<Branch<32, ()>>(), 64 * 33);
        assert_eq!(mem::size_of::<Branch<64, ()>>(), 64 * 65);
        assert_eq!(mem::size_of::<Branch<64, ()>>(), 64 * 65);
    }

    #[test]
    fn infix_size() {
        assert_eq!(mem::size_of::<Infix<14, ()>>(), 16 * 2);
        assert_eq!(mem::size_of::<Infix<30, ()>>(), 16 * 3);
        assert_eq!(mem::size_of::<Infix<46, ()>>(), 16 * 4);
        assert_eq!(mem::size_of::<Infix<62, ()>>(), 16 * 5);
    }
}
