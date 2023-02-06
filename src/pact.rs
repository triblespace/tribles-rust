mod branch;
mod empty;
mod leaf;
mod macros;
mod setops;

use branch::*;
use empty::*;
use leaf::*;
use macros::*;
//use setops::*;

use crate::bitset::ByteBitset;
use crate::bytetable;
use crate::bytetable::*;
use crate::query::{ByteCursor, CursorIterator, Peek};
use core::hash::Hasher;
use rand::thread_rng;
use rand::RngCore;
use siphasher::sip128::{Hasher128, SipHasher24};
use std::any::type_name;
use std::cmp::{max, min};
use std::fmt;
use std::fmt::Debug;
use std::mem;
use std::mem::ManuallyDrop;
use std::mem::{transmute, MaybeUninit};
use std::sync::Arc;
use std::sync::Once;

static mut SIP_KEY: [u8; 16] = [0; 16];
static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        bytetable::init();

        let mut rng = thread_rng();
        unsafe {
            rng.fill_bytes(&mut SIP_KEY[..]);
        }
    });
}

const HEAD_FRAGMENT_LEN: usize = 5;
const LEAF_FRAGMENT_LEN: usize = 14;

fn index_start(infix_start: usize, index: usize) -> usize {
    index - infix_start
}

fn copy_start(target: &mut [u8], source: &[u8], start_index: usize) {
    let target_len = target.len();
    let source_len = source.len();
    let used_len = min(source_len - start_index as usize, target_len);
    let target_range = &mut target[0..used_len];
    let source_range = &source[start_index..start_index as usize + used_len];
    target_range.copy_from_slice(source_range);
}
/*
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
*/

trait HeadVariant<const KEY_LEN: usize>: Sized {
    /// Returns a path byte fragment or all possible branch options
    /// at the given depth.
    fn peek(&self, at_depth: usize) -> Peek;

    /// Return the child stored at the provided depth under the provided key.
    /// Will return `self` when the fragment matches the key at the depth.
    fn get(&self, at_depth: usize, key: u8) -> Head<KEY_LEN>;

    /// Stores the provided key in the node. This returns a new node
    /// which may or may not share structure with the provided node.
    fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN>;

    /// Returns the number of leafs in the subtree under this node.
    fn count(&self) -> u64;

    /// Returns the xored sum of all hashes of leafs
    //  in the subtree under this node.
    fn hash(&self, prefix: &[u8; KEY_LEN]) -> u128;

    fn with_start(&self, _new_start_depth: usize, _key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        panic!("`with_start` not supported by {}", type_name::<Self>());
    }

    fn insert(&mut self, _key: &[u8; KEY_LEN], _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`insert` not supported by {}", type_name::<Self>());
    }

    fn reinsert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`reinsert` not supported by {}", type_name::<Self>());
    }

    fn grow(&self) -> Head<KEY_LEN> {
        panic!("`grow` not supported by {}", type_name::<Self>());
    }
}

#[derive(Debug)]
#[repr(C)]
struct Unknown {
    tag: HeadTag,
    start_depth: u8,
    key: u8,
    ignore: [MaybeUninit<u8>; 13],
}

#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
enum HeadTag {
    Empty = 0,
    Branch4,
    Branch8,
    Branch16,
    Branch32,
    Branch64,
    Branch128,
    Branch256,
    InlineLeaf,
    Leaf,
}

#[repr(C)]
pub union Head<const KEY_LEN: usize> {
    unknown: ManuallyDrop<Unknown>,
    empty: ManuallyDrop<Empty<KEY_LEN>>,
    branch4: ManuallyDrop<Branch4<KEY_LEN>>,
    branch8: ManuallyDrop<Branch8<KEY_LEN>>,
    branch16: ManuallyDrop<Branch16<KEY_LEN>>,
    branch32: ManuallyDrop<Branch32<KEY_LEN>>,
    branch64: ManuallyDrop<Branch64<KEY_LEN>>,
    branch128: ManuallyDrop<Branch128<KEY_LEN>>,
    branch256: ManuallyDrop<Branch256<KEY_LEN>>,
    inlineleaf: ManuallyDrop<InlineLeaf<KEY_LEN>>,
    leaf: ManuallyDrop<Leaf<KEY_LEN>>,
}

unsafe impl<const KEY_LEN: usize> ByteEntry for Head<KEY_LEN> {
    fn zeroed() -> Self {
        Empty::new().into()
    }

    fn key(&self) -> Option<u8> {
        unsafe {
            if self.unknown.tag == HeadTag::Empty {
                None
            } else {
                Some(self.unknown.key)
            }
        }
    }
}

impl<const KEY_LEN: usize> fmt::Debug for Head<KEY_LEN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        dispatch!(self, variant, variant.fmt(f))
    }
}

impl<const KEY_LEN: usize> Clone for Head<KEY_LEN> {
    fn clone(&self) -> Self {
        dispatch!(
            self,
            variant,
            Head::from(ManuallyDrop::into_inner(variant.clone()))
        )
    }
}

impl<const KEY_LEN: usize> Drop for Head<KEY_LEN> {
    fn drop(&mut self) {
        dispatch_mut!(self, variant, ManuallyDrop::drop(variant))
    }
}

impl<const KEY_LEN: usize> Default for Head<KEY_LEN> {
    fn default() -> Self {
        Empty::new().into()
    }
}

impl<const KEY_LEN: usize> Head<KEY_LEN> {
    fn count(&self) -> u64 {
        dispatch!(self, variant, variant.count())
    }

    fn with_start(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        dispatch!(self, variant, variant.with_start(new_start_depth, key))
    }

    fn peek(&self, at_depth: usize) -> Peek {
        dispatch!(self, variant, variant.peek(at_depth))
    }

    fn get(&self, at_depth: usize, key: u8) -> Self {
        dispatch!(self, variant, variant.get(at_depth, key))
    }

    fn put(&mut self, key: &[u8; KEY_LEN]) -> Self {
        dispatch_mut!(self, variant, variant.put(key))
    }

    fn insert(&mut self, key: &[u8; KEY_LEN], child: Self) -> Self {
        dispatch_mut!(self, variant, variant.insert(key, child))
    }

    fn reinsert(&mut self, child: Self) -> Self {
        dispatch_mut!(self, variant, variant.reinsert(child))
    }

    fn grow(&self) -> Self {
        dispatch!(self, variant, variant.grow())
    }

    fn hash(&self, prefix: &[u8; KEY_LEN]) -> u128 {
        dispatch!(self, variant, variant.hash(prefix))
    }
}

#[derive(Debug, Clone)]
pub struct PACT<const KEY_LEN: usize> {
    root: Head<KEY_LEN>,
}

impl<const KEY_LEN: usize> PACT<KEY_LEN>
where
    [Head<KEY_LEN>; KEY_LEN]: Sized,
{
    pub fn new() -> Self {
        PACT {
            root: Empty::new().into(),
        }
    }

    pub fn put(&mut self, key: [u8; KEY_LEN]) {
        self.root = self.root.put(&key);
    }

    pub fn len(&self) -> u64 {
        self.root.count()
    }

    pub fn cursor(&self) -> PACTCursor<KEY_LEN> {
        return PACTCursor::new(self);
    }
}

pub struct PACTCursor<const KEY_LEN: usize>
where
    [Head<KEY_LEN>; KEY_LEN]: Sized,
{
    depth: usize,
    path: [Head<KEY_LEN>; KEY_LEN],
}

impl<const KEY_LEN: usize> PACTCursor<KEY_LEN>
where
    [Head<KEY_LEN>; KEY_LEN]: Sized,
{
    pub fn new(tree: &PACT<KEY_LEN>) -> Self {
        let mut new = Self {
            depth: 0,
            path: unsafe { mem::zeroed() },
        };
        new.path[0] = tree.root.clone();
        return new;
    }
}

impl<const KEY_LEN: usize> ByteCursor for PACTCursor<KEY_LEN>
where
    [Head<KEY_LEN>; KEY_LEN]: Sized,
{
    fn peek(&self) -> Peek {
        self.path[self.depth].peek(self.depth)
    }

    fn push(&mut self, byte: u8) {
        self.path[self.depth + 1] = self.path[self.depth].get(self.depth, byte);
        self.depth += 1;
    }

    fn pop(&mut self) {
        self.path[self.depth] = unsafe { mem::zeroed() };
        self.depth -= 1;
    }

    fn segment_count(&self) -> u32 {
        return 0;
        //return self.path[self.depth].segment_count(self.depth);
    }
}

impl<const KEY_LEN: usize> IntoIterator for PACTCursor<KEY_LEN>
where
    [Head<KEY_LEN>; KEY_LEN]: Sized,
{
    type Item = [u8; KEY_LEN];
    type IntoIter = CursorIterator<Self, KEY_LEN>;

    fn into_iter(self) -> Self::IntoIter {
        CursorIterator::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head<64>>(), 16);
    }

    #[test]
    fn empty_tree() {
        init();

        let _tree = PACT::<64>::new();
    }

    #[test]
    fn tree_put_one() {
        init();

        const KEY_SIZE: usize = 64;
        let mut tree = PACT::<KEY_SIZE>::new();
        let key = [0; KEY_SIZE];
        tree.put(key);
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable4<Head<64>>>(), 64);
        assert_eq!(mem::size_of::<BranchBody4<64>>(), 64 * 3);
        assert_eq!(mem::size_of::<BranchBody8<64>>(), 64 * 4);
        assert_eq!(mem::size_of::<BranchBody16<64>>(), 64 * 6);
        assert_eq!(mem::size_of::<BranchBody32<64>>(), 64 * 10);
        assert_eq!(mem::size_of::<BranchBody64<64>>(), 64 * 18);
        assert_eq!(mem::size_of::<BranchBody128<64>>(), 64 * 34);
        assert_eq!(mem::size_of::<BranchBody256<64>>(), 64 * 66);
    }

    proptest! {
        #[test]
        fn tree_put(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64>::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(key);
            }
        }

        #[test]
        fn tree_len(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64>::new();
            let mut set = HashSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(key);
                set.insert(key);
            }
            prop_assert_eq!(set.len() as u64, tree.len())
        }

        #[test]
        fn tree_iter(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64>::new();
            let mut set = HashSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(key);
                set.insert(key);
            }
            let tree_set = HashSet::from_iter(tree.cursor().into_iter());
            prop_assert_eq!(set, tree_set);
        }
    }
}
