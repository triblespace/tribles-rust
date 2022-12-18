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

pub trait SizeLimited<const LIMIT: usize>: Sized {
    const UNUSED: usize = LIMIT - std::mem::size_of::<Self>();
}

impl<A: Sized, const LIMIT: usize> SizeLimited<LIMIT> for A {}

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

#[repr(packed(1), C)]
struct BranchHead<const KEY_LEN: usize, Value, const TABLE_SIZE: usize>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fragment: [u8; 5],
    start_depth: u8,
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
            fragment: self.fragment,
            start_depth: self.start_depth,
            end_depth: self.end_depth,
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

#[repr(C)]
struct PathBody<const KEY_LEN: usize, Value, const FRAGMENT_LEN: usize>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    child: Head<KEY_LEN, Value>,
    rc: AtomicU16,
    fragment: [u8; FRAGMENT_LEN],
}

#[repr(packed(1), C)]
struct PathHead<const KEY_LEN: usize, Value, const FRAGMENT_LEN: usize>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fragment: [u8; 5],
    start_depth: u8,
    end_depth: u8,
    ptr: NonNull<PathBody<KEY_LEN, Value, FRAGMENT_LEN>>,
    phantom: PhantomData<PathBody<KEY_LEN, Value, FRAGMENT_LEN>>,
}

impl<const KEY_LEN: usize, Value, const FRAGMENT_LEN: usize> PathHead<KEY_LEN, Value, FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn init(key: &[u8; KEY_LEN], start_depth: usize, child: Head<KEY_LEN, Value>) -> Self {
        unsafe {
            let end_depth = child.start_depth();
            let layout = Layout::new::<PathBody<KEY_LEN, Value, FRAGMENT_LEN>>();
            let path_body = alloc(layout) as *mut PathBody<KEY_LEN, Value, FRAGMENT_LEN>;
            if path_body.is_null() {
                panic!("Alloc error!");
            }
            path_body.write(PathBody {
                child: child,
                rc: AtomicU16::new(1),
                fragment: mem::zeroed(),
            });

            copy_end((*path_body).fragment.as_mut_slice(), &key[..], end_depth);

            let mut path_head = Self {
                fragment: unsafe { mem::zeroed() },
                start_depth: start_depth as u8,
                end_depth: end_depth as u8,
                ptr: NonNull::new_unchecked(path_body),
                phantom: PhantomData
            };
            
            copy_start(path_head.fragment.as_mut_slice(), &key[..], start_depth);

            return path_head;
        }
    }
}

impl<const KEY_LEN: usize, Value, const FRAGMENT_LEN: usize> Clone
    for PathHead<KEY_LEN, Value, FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn clone(&self) -> Self {
        Self {
            fragment: self.fragment,
            start_depth: self.start_depth,
            end_depth: self.end_depth,
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

//#[rustc_layout(debug)]
#[derive(Clone)]
#[repr(u8)]
enum Head<const KEY_LEN: usize, Value: SizeLimited<13> + Clone>
where
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    Empty {
        padding: [u8; 15],
    } = 0,
    Branch1 {
        head: BranchHead<KEY_LEN, Value, 1>,
    },
    Branch2 {
        head: BranchHead<KEY_LEN, Value, 2>,
    },
    Branch4 {
        head: BranchHead<KEY_LEN, Value, 4>,
    },
    Branch8 {
        head: BranchHead<KEY_LEN, Value, 8>,
    },
    Branch16 {
        head: BranchHead<KEY_LEN, Value, 16>,
    },
    Branch32 {
        head: BranchHead<KEY_LEN, Value, 32>,
    },
    Branch64 {
        head: BranchHead<KEY_LEN, Value, 64>,
    },
    Path14 {
        head: PathHead<KEY_LEN, Value, 14>,
    },
    Path30 {
        head: PathHead<KEY_LEN, Value, 30>,
    },
    Path46 {
        head: PathHead<KEY_LEN, Value, 46>,
    },
    Path62 {
        head: PathHead<KEY_LEN, Value, 62>,
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
    fn new_leaf(start_depth: usize, key: &[u8; KEY_LEN], value: Value) -> Self {
        let actual_start_depth = max(start_depth, Head::<KEY_LEN, Value>::LEAF_FRAGMENT_RANGE);

        let mut new_leaf = Self::Leaf {
            fragment: unsafe { mem::zeroed() },
            start_depth: actual_start_depth as u8,
            value: value.clone(),
        };

        if let Self::Leaf { mut fragment, .. } = new_leaf {
            copy_start(&mut fragment[..], &key[..], actual_start_depth);
        }

        return new_leaf;
    }

    fn wrap_path(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let expanded = self.expand(start_depth, key);

        let actual_start_depth = expanded.start_depth();
        if start_depth == actual_start_depth {
            return expanded;
        }

        let path_length = actual_start_depth - start_depth;

        if path_length <= 19 {
            return Self::Path14 {
                head: PathHead::<KEY_LEN, Value, 14>::init(&key, start_depth, expanded),
            };
        }

        if path_length <= 35 {
            return Self::Path30 {
                head: PathHead::<KEY_LEN, Value, 30>::init(&key, start_depth, expanded),
            };
        }

        if path_length <= 51 {
            return Self::Path46 {
                head: PathHead::<KEY_LEN, Value, 46>::init(&key, start_depth, expanded),
            };
        }

        if path_length <= 67 {
            return Self::Path62 {
                head: PathHead::<KEY_LEN, Value, 62>::init(&key, start_depth, expanded),
            };
        }

        panic!("Fragment too long for path to hold.");
    }

    fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        match self {
            Head::Empty { .. } => self,
            Head::Branch1 { .. } => self,
            Head::Branch1 { .. } => self,
            Head::Branch2 { .. } => self,
            Head::Branch4 { .. } => self,
            Head::Branch8 { .. } => self,
            Head::Branch16 { .. } => self,
            Head::Branch32 { .. } => self,
            Head::Branch64 { .. } => self,
            Head::Path14 { .. } => self,
            Head::Path30 { .. } => self,
            Head::Path46 { .. } => self,
            Head::Path62 { .. } => self,
            Head::Leaf { .. } => self,
        }
    }

    fn start_depth(&self) -> usize {
        (match self {
            Head::Empty { .. } => 0,
            Head::Branch1 { head } => head.start_depth,
            Head::Branch2 { head } => head.start_depth,
            Head::Branch4 { head } => head.start_depth,
            Head::Branch8 { head } => head.start_depth,
            Head::Branch1 { head } => head.start_depth,
            Head::Branch16 { head } => head.start_depth,
            Head::Branch32 { head } => head.start_depth,
            Head::Branch64 { head } => head.start_depth,
            Head::Path14 { head } => head.start_depth,
            Head::Path30 { head } => head.start_depth,
            Head::Path46 { head } => head.start_depth,
            Head::Path62 { head } => head.start_depth,
            Head::Leaf { start_depth, .. } => *start_depth,
        }) as usize
    }
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
            Head::Branch1 { head } => Some(head.fragment[0]),
            Head::Branch1 { head } => Some(head.fragment[0]),
            Head::Branch2 { head } => Some(head.fragment[0]),
            Head::Branch4 { head } => Some(head.fragment[0]),
            Head::Branch8 { head } => Some(head.fragment[0]),
            Head::Branch16 { head } => Some(head.fragment[0]),
            Head::Branch32 { head } => Some(head.fragment[0]),
            Head::Branch64 { head } => Some(head.fragment[0]),
            Head::Path14 { head } => Some(head.fragment[0]),
            Head::Path30 { head } => Some(head.fragment[0]),
            Head::Path46 { head } => Some(head.fragment[0]),
            Head::Path62 { head } => Some(head.fragment[0]),
            Head::Leaf { fragment, .. } => Some(fragment[0]),
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
            head: Head::Empty {
                padding: unsafe { mem::zeroed() },
            },
        }
    }

    pub fn put(&mut self, key: [u8; KEY_LEN], value: Value) {
        if let Head::Empty { .. } = self.head {
            self.head = Head::<KEY_LEN, Value>::new_leaf(0, &key, value).wrap_path(0, &key);
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
