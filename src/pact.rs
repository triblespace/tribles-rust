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
use std::marker::PhantomData;

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

type SharedKey<const KEY_LEN: usize> = Arc<[u8; KEY_LEN]>;

pub trait KeyProperties<const KEY_LEN: usize>: Copy + Clone + Debug {
    fn reorder(at_depth: usize) -> usize;
}

#[derive(Copy, Clone, Debug)]
pub struct IdentityOrder {}

impl<const KEY_LEN: usize> KeyProperties<KEY_LEN> for IdentityOrder {
    fn reorder(depth: usize) -> usize {
        depth
    }
}

trait HeadVariant<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>>: Sized {
    /// Returns a path byte fragment or all possible branch options
    /// at the given depth.
    fn peek(&self, at_depth: usize) -> Peek;

    /// Return the child stored at the provided depth under the provided key.
    /// Will return `self` when the fragment matches the key at the depth.
    fn get(&self, at_depth: usize, key: u8) -> Head<KEY_LEN, K>;

    /// Stores the provided key in the node. This returns a new node
    /// which may or may not share structure with the provided node.
    fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Head<KEY_LEN, K>;

    /// Returns the number of leafs in the subtree under this node.
    fn count(&self) -> u64;

    /// Returns the xored sum of all hashes of leafs
    //  in the subtree under this node.
    fn hash(&self, prefix: &[u8; KEY_LEN]) -> u128;

    fn with_start(&self, _new_start_depth: usize, _key: &[u8; KEY_LEN]) -> Head<KEY_LEN, K> {
        panic!(
            "`with_start` not supported by {}",
            type_name::<Self>()
        );
    }

    fn insert(&mut self, _key: &[u8; KEY_LEN], _child: Head<KEY_LEN, K>) -> Head<KEY_LEN, K> {
        panic!("`insert` not supported by {}", type_name::<Self>());
    }

    fn reinsert(&mut self, _child: Head<KEY_LEN, K>) -> Head<KEY_LEN, K> {
        panic!("`reinsert` not supported by {}", type_name::<Self>());
    }

    fn grow(&self) -> Head<KEY_LEN, K> {
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
    Leaf,
    SharedLeaf,
}

#[repr(C)]
pub union Head<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> {
    unknown: ManuallyDrop<Unknown>,
    empty: ManuallyDrop<Empty<KEY_LEN, K>>,
    branch4: ManuallyDrop<Branch4<KEY_LEN, K>>,
    branch8: ManuallyDrop<Branch8<KEY_LEN, K>>,
    branch16: ManuallyDrop<Branch16<KEY_LEN, K>>,
    branch32: ManuallyDrop<Branch32<KEY_LEN, K>>,
    branch64: ManuallyDrop<Branch64<KEY_LEN, K>>,
    branch128: ManuallyDrop<Branch128<KEY_LEN, K>>,
    branch256: ManuallyDrop<Branch256<KEY_LEN, K>>,
    leaf: ManuallyDrop<Leaf<KEY_LEN, K>>,
    sharedleaf: ManuallyDrop<SharedLeaf<KEY_LEN, K>>,
}

unsafe impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> ByteEntry for Head<KEY_LEN, K> {
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

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> fmt::Debug for Head<KEY_LEN, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        dispatch!(self, variant, variant.fmt(f))
    }
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> Clone for Head<KEY_LEN, K> {
    fn clone(&self) -> Self {
        dispatch!(
            self,
            variant,
            Head::from(ManuallyDrop::into_inner(variant.clone()))
        )
    }
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> Drop for Head<KEY_LEN, K> {
    fn drop(&mut self) {
        dispatch_mut!(self, variant, ManuallyDrop::drop(variant))
    }
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> Default for Head<KEY_LEN, K> {
    fn default() -> Self {
        Empty::new().into()
    }
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> Head<KEY_LEN, K> {
    fn count(&self) -> u64 {
        dispatch!(self, variant, variant.count())
    }

    fn with_start(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, K> {
        dispatch!(
            self,
            variant,
            variant.with_start(new_start_depth, key)
        )
    }

    fn peek(&self, at_depth: usize) -> Peek {
        dispatch!(self, variant, variant.peek(at_depth))
    }

    fn get(&self, at_depth: usize, key: u8) -> Self {
        dispatch!(self, variant, variant.get(at_depth, key))
    }

    fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Self {
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
pub struct PACT<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> {
    root: Head<KEY_LEN, K>,
}

impl<const KEY_LEN: usize, K> PACT<KEY_LEN, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
{
    pub fn new() -> Self {
        PACT {
            root: Empty::new().into(),
        }
    }

    pub fn put(&mut self, key: &SharedKey<KEY_LEN>) {
        self.root = self.root.put(key);
    }

    pub fn len(&self) -> u64 {
        self.root.count()
    }

    pub fn cursor(&self) -> PACTCursor<KEY_LEN, K> {
        return PACTCursor::new(self);
    }
}

pub struct PACTCursor<const KEY_LEN: usize, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
{
    depth: usize,
    path: [Head<KEY_LEN, K>; KEY_LEN],
}

impl<const KEY_LEN: usize, K> PACTCursor<KEY_LEN, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
{
    pub fn new(tree: &PACT<KEY_LEN, K>) -> Self {
        let mut new = Self {
            depth: 0,
            path: unsafe { mem::zeroed() },
        };
        new.path[0] = tree.root.clone();
        return new;
    }
}

impl<const KEY_LEN: usize, K> ByteCursor for PACTCursor<KEY_LEN, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
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

impl<const KEY_LEN: usize, K> IntoIterator for PACTCursor<KEY_LEN, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
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
        assert_eq!(mem::size_of::<Head<64, IdentityOrder>>(), 16);
    }

    #[test]
    fn empty_tree() {
        init();

        let _tree = PACT::<64, IdentityOrder>::new();
    }

    #[test]
    fn tree_put_one() {
        init();

        const KEY_SIZE: usize = 64;
        let mut tree = PACT::<KEY_SIZE, IdentityOrder>::new();
        let key = Arc::new([0; KEY_SIZE]);
        tree.put(&key);
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable4<Head<64, IdentityOrder>>>(), 64);
        assert_eq!(mem::size_of::<BranchBody4<64, IdentityOrder>>(), 64 * 2);
        assert_eq!(mem::size_of::<BranchBody8<64, IdentityOrder>>(), 64 * 3);
        assert_eq!(mem::size_of::<BranchBody16<64, IdentityOrder>>(), 64 * 5);
        assert_eq!(mem::size_of::<BranchBody32<64, IdentityOrder>>(), 64 * 9);
        assert_eq!(mem::size_of::<BranchBody64<64, IdentityOrder>>(), 64 * 17);
        assert_eq!(mem::size_of::<BranchBody128<64, IdentityOrder>>(), 64 * 33);
        assert_eq!(mem::size_of::<BranchBody256<64, IdentityOrder>>(), 64 * 65);
    }

    proptest! {
        #[test]
        fn tree_put(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64, IdentityOrder>::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(&Arc::new(key));
            }
        }

        #[test]
        fn tree_len(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64, IdentityOrder>::new();
            let mut set = HashSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(&Arc::new(key));
                set.insert(key);
            }
            prop_assert_eq!(set.len() as u64, tree.len())
        }

        #[test]
        fn tree_iter(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64, IdentityOrder>::new();
            let mut set = HashSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(&Arc::new(key));
                set.insert(key);
            }
            let tree_set = HashSet::from_iter(tree.cursor().into_iter());
            prop_assert_eq!(set, tree_set);
        }
    }
}
