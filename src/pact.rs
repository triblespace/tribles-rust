use crate::bitset::ByteBitset;
use crate::bytetable::{ByteEntry, ByteTable};
use siphasher::sip128::{Hasher128, SipHasher24};
use std::alloc::{alloc, dealloc, Layout};
use std::cmp::{max, min};
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU16;

pub trait SizeLimited<const LIMIT: usize>: Sized {
    const UNUSED: usize = LIMIT - std::mem::size_of::<Self>();
}

impl<A: Sized, const LIMIT: usize> SizeLimited<LIMIT> for A {}

#[repr(C)]
struct Branch<const KEY_LEN: usize, Value: SizeLimited<13> + Clone, const TABLE_SIZE: usize>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    leaf_count: u64,
    rc: AtomicU16,
    segment_count: u32, //TODO: increase this to a u48
    node_hash: u128,
    child_set: ByteBitset,
    children: ByteTable<TABLE_SIZE, Head<KEY_LEN, Value>>,
}

#[repr(C)]
struct Path<const KEY_LEN: usize, Value: SizeLimited<13> + Clone, const INFIX_LEN: usize>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    child: Head<KEY_LEN, Value>,
    rc: AtomicU16,
    fragment: [u8; INFIX_LEN],
}

//#[rustc_layout(debug)]
#[derive(Clone, Debug)]
#[repr(u8)]
enum Head<const KEY_LEN: usize, Value: SizeLimited<13> + Clone>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    Empty {
        padding: [u8; 15],
    } = 0,
    Branch1 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<KEY_LEN, Value, 1>>,
        phantom: PhantomData<Branch<KEY_LEN, Value, 1>>,
    },
    Branch2 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<KEY_LEN, Value, 2>>,
        phantom: PhantomData<Branch<KEY_LEN, Value, 2>>,
    },
    Branch4 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<KEY_LEN, Value, 4>>,
        phantom: PhantomData<Branch<KEY_LEN, Value, 4>>,
    },
    Branch8 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<KEY_LEN, Value, 8>>,
        phantom: PhantomData<Branch<KEY_LEN, Value, 8>>,
    },
    Branch16 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<KEY_LEN, Value, 16>>,
        phantom: PhantomData<Branch<KEY_LEN, Value, 16>>,
    },
    Branch32 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<KEY_LEN, Value, 32>>,
        phantom: PhantomData<Branch<KEY_LEN, Value, 32>>,
    },
    Branch64 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Branch<KEY_LEN, Value, 64>>,
        phantom: PhantomData<Branch<KEY_LEN, Value, 64>>,
    },
    Path14 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Path<KEY_LEN, Value, 14>>,
        phantom: PhantomData<Path<KEY_LEN, Value, 14>>,
    },
    Path30 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Path<KEY_LEN, Value, 30>>,
        phantom: PhantomData<Path<KEY_LEN, Value, 30>>,
    },
    Path46 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Path<KEY_LEN, Value, 46>>,
        phantom: PhantomData<Path<KEY_LEN, Value, 46>>,
    },
    Path62 {
        fragment: [u8; 5],
        start_depth: u8,
        branch_depth: u8,
        ptr: NonNull<Path<KEY_LEN, Value, 62>>,
        phantom: PhantomData<Path<KEY_LEN, Value, 62>>,
    },
    Leaf {
        fragment: [u8; <Value as SizeLimited<13>>::UNUSED + 1],
        start_depth: u8,
        value: Value,
    },
}

impl<const KEY_LEN: usize, Value> Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const LEAF_FRAGMENT_RANGE: usize = KEY_LEN - <Value as SizeLimited<13>>::UNUSED + 1;
}

unsafe impl<const KEY_LEN: usize, Value: SizeLimited<13> + Clone> ByteEntry for Head<KEY_LEN, Value>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn zeroed() -> Self {
        return Head::Empty {
            padding: unsafe { mem::zeroed() },
        };
    }

    fn key(&self) -> Option<u8> {
        match self {
            Head::Empty { .. } => None,
            Head::Branch1 { fragment, .. } => Some(fragment[0]),
            Head::Branch1 { fragment, .. } => Some(fragment[0]),
            Head::Branch2 { fragment, .. } => Some(fragment[0]),
            Head::Branch4 { fragment, .. } => Some(fragment[0]),
            Head::Branch8 { fragment, .. } => Some(fragment[0]),
            Head::Branch16 { fragment, .. } => Some(fragment[0]),
            Head::Branch32 { fragment, .. } => Some(fragment[0]),
            Head::Branch64 { fragment, .. } => Some(fragment[0]),
            Head::Path14 { fragment, .. } => Some(fragment[0]),
            Head::Path30 { fragment, .. } => Some(fragment[0]),
            Head::Path46 { fragment, .. } => Some(fragment[0]),
            Head::Path62 { fragment, .. } => Some(fragment[0]),
            Head::Leaf { fragment, .. } => Some(fragment[0]),
            _ => None,
        }
    }
}

/*
fn copy_end(target: []u8, source: []const u8, end_index: u8) void {
    const used_len = @min(end_index, target.len);
    mem.copy(u8, target[target.len - used_len ..], source[end_index - used_len .. end_index]);
}
*/

fn copy_start(target: &mut [u8], source: &[u8], start_index: usize) {
    let used_len = min(source.len() - start_index, target.len());
    target[0..used_len].copy_from_slice(&source[start_index..start_index + used_len]);
}

/// We want to copy the last bytes of the key into the leaf fragment.
/// Note how the position of the fragment changes relative to the key when the
/// start_depth is outside of the range that can be covered by the fragment.
///
///
/// Case: start_depth < fragment_range                     ┌──────────┐
///    ┌───────────────────────────────────────────────────┤ fragment │
///    │                             key                   └──────────┤
///    └──────────────────────────────────────▲────────────▲──────────▲
///                               start_depth─┘            │  KEY_LEN─┘
///                                         fragment_range─┘
///
///
/// Case: start_depth > fragment_range                          ┌──────────┐
///    ┌────────────────────────────────────────────────────────┤ fragment │
///    │                             key                        └─────┬────┘
///    └───────────────────────────────────────────────────▲────▲─────▲
///                                         fragment_range─┘    │     │
///                                                 start_depth─┘     │
///                                                           KEY_LEN─┘
///
fn new_leaf<const KEY_LEN: usize, Value>(
    start_depth: usize,
    key: &[u8; KEY_LEN],
    value: Value,
) -> Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    let actual_start_depth = max(start_depth, Head::<KEY_LEN, Value>::LEAF_FRAGMENT_RANGE);

    let mut new_leaf = Head::Leaf {
        fragment: unsafe { mem::zeroed() },
        start_depth: actual_start_depth as u8,
        value: value.clone(),
    };

    if let Head::Leaf { mut fragment, .. } = new_leaf {
        copy_start(&mut fragment[..], &key[..], actual_start_depth);
    }

    return new_leaf;
}

pub struct Tree<const KEY_LEN: usize, Value: SizeLimited<13> + Clone>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    head: Head<KEY_LEN, Value>,
}

impl<const KEY_LEN: usize, Value: SizeLimited<13> + Clone> Tree<KEY_LEN, Value>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const KEY_LEN_CHECK: usize = KEY_LEN - 64;

    pub fn new() -> Self {
        Tree {
            head: Head::Empty {
                padding: unsafe { mem::zeroed() },
            },
        }
    }

    pub fn put(&mut self, key: [u8; KEY_LEN], value: Value) {
        if let Head::Empty { .. } = self.head {
            self.head = new_leaf(0, &key, value);
            //self.child = wrap_path(0, key, new_leaf(0, key, value));
        } else {
            //self.child = try self.child.put(0, key, value, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head<64, ()>>(), 16);
        assert_eq!(mem::size_of::<Head<64, u64>>(), 16);
    }

    #[test]
    fn leaf_fragment_size() {
        let head_twig = Head::<64, ()>::Leaf {
            fragment: unsafe { mem::zeroed() },
            start_depth: 0,
            value: (),
        };
        if let Head::<64, ()>::Leaf { fragment, .. } = head_twig {
            assert_eq!(fragment.len(), 14);
        }

        let head = Head::<64, u64>::Leaf {
            fragment: unsafe { mem::zeroed() },
            start_depth: 0,
            value: 0,
        };
        if let Head::<64, u64>::Leaf { fragment, .. } = head {
            assert_eq!(fragment.len(), 6);
        }
    }

    #[test]
    fn empty_tree() {
        let tree = Tree::<64, ()>::new();
    }

    #[test]
    fn tree_insert_one() {
        const KEY_SIZE: usize = 64;
        let mut tree = Tree::<KEY_SIZE, ()>::new();
        let key = [0; KEY_SIZE];
        tree.put(key, ());
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable<1, Head<64, ()>>>(), 64);
        assert_eq!(mem::size_of::<Branch<64, (), 1>>(), 64 * 2);
        assert_eq!(mem::size_of::<Branch<64, (), 2>>(), 64 * 3);
        assert_eq!(mem::size_of::<Branch<64, (), 4>>(), 64 * 5);
        assert_eq!(mem::size_of::<Branch<64, (), 8>>(), 64 * 9);
        assert_eq!(mem::size_of::<Branch<64, (), 16>>(), 64 * 17);
        assert_eq!(mem::size_of::<Branch<64, (), 32>>(), 64 * 33);
        assert_eq!(mem::size_of::<Branch<64, (), 64>>(), 64 * 65);
    }

    #[test]
    fn fragment_size() {
        assert_eq!(mem::size_of::<Path<64, (), 14>>(), 16 * 2);
        assert_eq!(mem::size_of::<Path<64, (), 30>>(), 16 * 3);
        assert_eq!(mem::size_of::<Path<64, (), 46>>(), 16 * 4);
        assert_eq!(mem::size_of::<Path<64, (), 62>>(), 16 * 5);
    }
}
