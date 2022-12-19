use crate::bitset::ByteBitset;
use crate::bytetable::{ByteEntry, ByteTable};
use siphasher::sip128::{Hasher128, SipHasher24};
use std::alloc::{alloc, dealloc, realloc, Layout};
use std::cmp::{max, min};
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU16;
use std::mem::ManuallyDrop;

pub trait SizeLimited<const LIMIT: usize>: Sized {
    const UNUSED: usize = LIMIT - std::mem::size_of::<Self>();
}

impl<A: Sized, const LIMIT: usize> SizeLimited<LIMIT> for A {}

const HEAD_FRAGMENT_LEN: usize = 5;

#[repr(C)]
struct UnknownHead {
    tag: HeadTag,
    start_depth: u8,
    key: u8,
    padding: [u8; 13],
}

#[repr(C)]
struct EmptyHead {
    tag: HeadTag,
    padding: [u8; 15],
}

impl EmptyHead {
    const TAG: HeadTag = HeadTag::Empty;

    fn new() -> Self {
        unsafe { mem::zeroed() }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
struct LeafHead<const KEY_LEN: usize, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; <Value as SizeLimited<13>>::UNUSED + 1],
    value: Value,
}

impl<const KEY_LEN: usize, Value> LeafHead<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const TAG: HeadTag = HeadTag::Leaf;
    const FRAGMENT_LEN: usize = KEY_LEN - <Value as SizeLimited<13>>::UNUSED + 1;

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
    fn new(start_depth: usize, key: &[u8; KEY_LEN], value: Value) -> Self {
        let actual_start_depth = max(start_depth, Self::FRAGMENT_LEN);

        let mut leaf_head = Self {
            tag: Self::TAG,
            start_depth: actual_start_depth as u8,
            fragment: unsafe { mem::zeroed() },
            value: value.clone(),
        };

        copy_start(leaf_head.fragment.as_mut_slice(), &key[..], actual_start_depth);

        return leaf_head;
    }
}

#[repr(C)]
struct BranchBody<const KEY_LEN: usize, Value, const TABLE_SIZE: usize>
where
    Value: SizeLimited<13> + Clone,
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
struct BranchHead<const KEY_LEN: usize, Value, const TABLE_SIZE: usize>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; HEAD_FRAGMENT_LEN],
    end_depth: u8,
    ptr: NonNull<BranchBody<KEY_LEN, Value, TABLE_SIZE>>,
    phantom: PhantomData<BranchBody<KEY_LEN, Value, TABLE_SIZE>>,
}

impl<const KEY_LEN: usize, Value, const TABLE_SIZE: usize> Clone
    for BranchHead<KEY_LEN, Value, TABLE_SIZE>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn clone(&self) -> Self {
        Self {
            tag: Self::TAG,
            start_depth: self.start_depth,
            fragment: self.fragment,
            end_depth: self.end_depth,
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<const KEY_LEN: usize, Value, const TABLE_SIZE: usize> BranchHead<KEY_LEN, Value, TABLE_SIZE>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const TAG: HeadTag = match TABLE_SIZE {
        1 => HeadTag::Branch1,
        2 => HeadTag::Branch2,
        4 => HeadTag::Branch4,
        8 => HeadTag::Branch8,
        16 => HeadTag::Branch16,
        32 => HeadTag::Branch32,
        64 => HeadTag::Branch64,
        l => panic!("invalid path length"),
    };
}

#[repr(C)]
struct PathBody<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    child: Head<KEY_LEN, Value>,
    rc: AtomicU16,
    fragment: [u8; BODY_FRAGMENT_LEN],
}

#[repr(C)]
struct PathHead<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; HEAD_FRAGMENT_LEN],
    end_depth: u8,
    ptr: NonNull<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>,
    phantom: PhantomData<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>,
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize> PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const TAG: HeadTag = match BODY_FRAGMENT_LEN {
        14 => HeadTag::Path14,
        30 => HeadTag::Path30,
        46 => HeadTag::Path46,
        62 => HeadTag::Path62,
        l => panic!("invalid path length"),
    };
    const FRAGMENT_LEN: usize = BODY_FRAGMENT_LEN + HEAD_FRAGMENT_LEN;

    fn init(key: &[u8; KEY_LEN], start_depth: usize, child: Head<KEY_LEN, Value>) -> Self {
        unsafe {
            let end_depth = child.start_depth();
            let layout = Layout::new::<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>();
            let path_body = alloc(layout) as *mut PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>;
            if path_body.is_null() {
                panic!("Alloc error!");
            }
            path_body.write(PathBody {
                child: child,
                rc: AtomicU16::new(1),
                fragment: mem::zeroed(),
            });

            copy_end((*path_body).fragment.as_mut_slice(), &key[..], end_depth);

            let actual_start_depth = max(start_depth as isize, end_depth as isize - Self::FRAGMENT_LEN as isize) as usize;

            let mut path_head = Self {
                tag: Self::TAG,
                start_depth: actual_start_depth as u8,
                fragment: unsafe { mem::zeroed() },
                end_depth: end_depth as u8,
                ptr: NonNull::new_unchecked(path_body),
                phantom: PhantomData
            };
            
            copy_start(path_head.fragment.as_mut_slice(), &key[..], actual_start_depth);

            return path_head;
        }
    }

    fn expand(&mut self, start_depth: usize, key: &[u8; KEY_LEN]) {
        let actual_start_depth = max(start_depth as isize, self.end_depth as isize - Self::FRAGMENT_LEN as isize) as usize;
        self.start_depth = actual_start_depth as u8;
        copy_start(self.fragment.as_mut_slice(), &key[..], actual_start_depth);
    }
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize> Clone
    for PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn clone(&self) -> Self {
        Self {
            tag: Self::TAG,
            start_depth: self.start_depth,
            fragment: self.fragment,
            end_depth: self.end_depth,
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
#[repr(u8)]
enum HeadTag {
    Empty = 0,
    Branch1,
    Branch2,
    Branch4,
    Branch8,
    Branch16,
    Branch32,
    Branch64,
    Path14,
    Path30,
    Path46,
    Path62,
    Leaf,
}

union Head<const KEY_LEN: usize, Value: SizeLimited<13> + Clone>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    unknown: ManuallyDrop<UnknownHead>,
    empty: ManuallyDrop<EmptyHead>,
    branch1: ManuallyDrop<BranchHead<KEY_LEN, Value, 1>>,
    branch2: ManuallyDrop<BranchHead<KEY_LEN, Value, 2>>,
    branch4: ManuallyDrop<BranchHead<KEY_LEN, Value, 4>>,
    branch8: ManuallyDrop<BranchHead<KEY_LEN, Value, 8>>,
    branch16: ManuallyDrop<BranchHead<KEY_LEN, Value, 16>>,
    branch32: ManuallyDrop<BranchHead<KEY_LEN, Value, 32>>,
    branch64: ManuallyDrop<BranchHead<KEY_LEN, Value, 64>>,
    path14: ManuallyDrop<PathHead<KEY_LEN, Value, 14>>,
    path30: ManuallyDrop<PathHead<KEY_LEN, Value, 30>>,
    path46: ManuallyDrop<PathHead<KEY_LEN, Value, 46>>,
    path62: ManuallyDrop<PathHead<KEY_LEN, Value, 62>>,
    leaf: ManuallyDrop<LeafHead<KEY_LEN, Value>>,
}

impl<const KEY_LEN: usize, Value> Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn wrap_path(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let mut expanded = self;
        expanded.expand(start_depth, key);

        let actual_start_depth = expanded.start_depth();
        if start_depth == actual_start_depth {
            return expanded;
        }

        let path_length = actual_start_depth - start_depth;

        if path_length <= PathHead::<KEY_LEN, Value, 14>::FRAGMENT_LEN {
            return Self {
                path14: ManuallyDrop::new(PathHead::<KEY_LEN, Value, 14>::init(&key, start_depth, expanded)),
            };
        }

        if path_length <= PathHead::<KEY_LEN, Value, 30>::FRAGMENT_LEN {
            return Self {
                path30: ManuallyDrop::new(PathHead::<KEY_LEN, Value, 30>::init(&key, start_depth, expanded)),
            };
        }

        if path_length <= PathHead::<KEY_LEN, Value, 46>::FRAGMENT_LEN {
            return Self {
                path46: ManuallyDrop::new(PathHead::<KEY_LEN, Value, 46>::init(&key, start_depth, expanded)),
            };
        }

        if path_length <= PathHead::<KEY_LEN, Value, 62>::FRAGMENT_LEN {
            return Self {
                path62: ManuallyDrop::new(PathHead::<KEY_LEN, Value, 62>::init(&key, start_depth, expanded)),
            };
        }

        panic!("Fragment too long for path to hold.");
    }

    fn expand(&mut self, start_depth: usize, key: &[u8; KEY_LEN]) {
        /*
        match self {
            Head::Empty { .. } => return,
            Head::Branch1 { .. } => self.head.expand(start_depth, key),
            Head::Branch1 { .. } => self.head.expand(start_depth, key),
            Head::Branch2 { .. } => self.head.expand(start_depth, key),
            Head::Branch4 { .. } => self.head.expand(start_depth, key),
            Head::Branch8 { .. } => self.head.expand(start_depth, key),
            Head::Branch16 { .. } => self.head.expand(start_depth, key),
            Head::Branch32 { .. } => self.head.expand(start_depth, key),
            Head::Branch64 { .. } => self.head.expand(start_depth, key),
            Head::Path14 { .. } => self.head.expand(start_depth, key),
            Head::Path30 { .. } => self.head.expand(start_depth, key),
            Head::Path46 { .. } => self.head.expand(start_depth, key),
            Head::Path62 { .. } => self.head.expand(start_depth, key),
            Head::Leaf { .. } => self.head.expand(start_depth, key)
        }
        */
    }

    fn start_depth(&self) -> usize {
        unsafe {self.unknown.start_depth as usize}
    }
}

unsafe impl<const KEY_LEN: usize, Value: SizeLimited<13> + Clone> ByteEntry for Head<KEY_LEN, Value>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn zeroed() -> Self {
        Self {empty: ManuallyDrop::new(EmptyHead::new())}
    }

    fn key(&self) -> Option<u8> {
        unsafe {
            if self.unknown.tag == EmptyHead::TAG {
                None
            } else {
                Some(self.unknown.key)
            }
        }
    }
}

impl<const KEY_LEN: usize, Value: SizeLimited<13> + Clone> Clone for Head<KEY_LEN, Value>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn clone(&self) -> Self {
        unsafe {
            match self.unknown.tag {
                Empty => Self { empty: ManuallyDrop::new(EmptyHead::new()) },
                Leaf => Self { leaf: self.leaf.clone() },
                Path14 => Self { path14: self.path14.clone() },
                Path30 => Self { path30: self.path30.clone() },
                Path46 => Self { path46: self.path46.clone() },
                Path62 => Self { path62: self.path62.clone() },
                Branch1 => Self { branch1: self.branch1.clone() },
                Branch2 => Self { branch2: self.branch2.clone() },
                Branch4 => Self { branch4: self.branch4.clone() },
                Branch8 => Self { branch8: self.branch8.clone() },
                Branch16 => Self { branch16: self.branch16.clone() },
                Branch32 => Self { branch32: self.branch32.clone() },
                Branch64 => Self { branch64: self.branch64.clone() },
            }
        }
    }
}

impl<const KEY_LEN: usize, Value: SizeLimited<13> + Clone> Drop for Head<KEY_LEN, Value>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn drop(&mut self) {
        unsafe {
            match self.unknown.tag {
                Empty => {},
                Leaf => ManuallyDrop::drop(&mut self.leaf),
                Path14 => ManuallyDrop::drop(&mut self.path14),
                Path30 => ManuallyDrop::drop(&mut self.path30),
                Path46 => ManuallyDrop::drop(&mut self.path46),
                Path62 => ManuallyDrop::drop(&mut self.path62),
                Branch1 => ManuallyDrop::drop(&mut self.branch1),
                Branch2 => ManuallyDrop::drop(&mut self.branch2),
                Branch4 => ManuallyDrop::drop(&mut self.branch4),
                Branch8 => ManuallyDrop::drop(&mut self.branch8),
                Branch16 => ManuallyDrop::drop(&mut self.branch16),
                Branch32 => ManuallyDrop::drop(&mut self.branch32),
                Branch64 => ManuallyDrop::drop(&mut self.branch64),
            }
        }
    }
}

fn copy_end(target: &mut [u8], source: &[u8], end_index: usize) {
    let target_len = target.len();
    let used_len = min(end_index, target_len);
    let target_range = &mut target[target_len - used_len..];
    let source_range = & source[end_index - used_len..end_index];
    target_range.copy_from_slice(source_range);
}

fn copy_start(target: &mut [u8], source: &[u8], start_index: usize) {
    let target_len = target.len();
    let source_len = source.len();
    let used_len = min(source_len - start_index, target_len);
    let target_range = &mut target[0..used_len];
    let source_range = & source[start_index..start_index + used_len];
    target_range.copy_from_slice(source_range);
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
            head: Head {empty: ManuallyDrop::new(EmptyHead::new())},
        }
    }

    pub fn put(&mut self, key: [u8; KEY_LEN], value: Value) {
        if unsafe { self.head.unknown.tag } == HeadTag::Empty {
            self.head = (Head::<KEY_LEN, Value> {
                            leaf: ManuallyDrop::new(LeafHead::<KEY_LEN, Value>::new(0, &key, value))
                        }).wrap_path(0, &key);
        } else {
            //self.child = self.child.put(0, &key, value, true);
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
        let twig = Head::<64, ()> {
            leaf: ManuallyDrop::new(LeafHead::<64, ()> {
                tag: LeafHead::<64, ()>::TAG,
                fragment: unsafe { mem::zeroed() },
                start_depth: 0,
                value: (),
            })
        };

        assert_eq!(unsafe { twig.leaf.fragment.len() }, 14);

        let leaf = Head::<64, u64> {
            leaf: ManuallyDrop::new(LeafHead::<64, u64> {
                tag: LeafHead::<64, u64>::TAG,
                fragment: unsafe { mem::zeroed() },
                start_depth: 0,
                value: 0,
            })
        };
        assert_eq!(unsafe {leaf.leaf.fragment.len()}, 6);
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
        assert_eq!(mem::size_of::<BranchBody<64, (), 1>>(), 64 * 2);
        assert_eq!(mem::size_of::<BranchBody<64, (), 2>>(), 64 * 3);
        assert_eq!(mem::size_of::<BranchBody<64, (), 4>>(), 64 * 5);
        assert_eq!(mem::size_of::<BranchBody<64, (), 8>>(), 64 * 9);
        assert_eq!(mem::size_of::<BranchBody<64, (), 16>>(), 64 * 17);
        assert_eq!(mem::size_of::<BranchBody<64, (), 32>>(), 64 * 33);
        assert_eq!(mem::size_of::<BranchBody<64, (), 64>>(), 64 * 65);
    }

    #[test]
    fn fragment_size() {
        assert_eq!(mem::size_of::<PathBody<64, (), 14>>(), 16 * 2);
        assert_eq!(mem::size_of::<PathBody<64, (), 30>>(), 16 * 3);
        assert_eq!(mem::size_of::<PathBody<64, (), 46>>(), 16 * 4);
        assert_eq!(mem::size_of::<PathBody<64, (), 62>>(), 16 * 5);
    }
}
