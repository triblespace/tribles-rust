//Persitent Adaptive Cuckoo Trie
// Should be renamed to PATCH
// Persistent Adaptive Trie with Compression and Hashing

mod branch;
mod bytecursor;
mod empty;
mod leaf;
mod macros;
mod paddingcursor;
mod setops;

use branch::*;
use empty::*;
use leaf::*;
use macros::*;

use crate::bitset::ByteBitset;
use crate::bytetable;
use crate::bytetable::*;
use core::hash::Hasher;
use rand::thread_rng;
use rand::RngCore;
use siphasher::sip128::{Hasher128, SipHasher24};
use std::any::type_name;
use std::cmp::min;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::mem::{transmute, MaybeUninit};
use std::sync::Once;
use triomphe::Arc;

static mut SIP_KEY: [u8; 16] = [0; 16];
static INIT: Once = Once::new();

const HEAD_FRAGMENT_LEN: usize = 5;
const LEAF_FRAGMENT_LEN: usize = 6;

pub fn init() {
    INIT.call_once(|| {
        bytetable::init();

        let mut rng = thread_rng();
        unsafe {
            rng.fill_bytes(&mut SIP_KEY[..]);
        }
    });
}

pub enum Peek {
    Fragment(u8),
    Branch(ByteBitset),
}

type SharedKey<const KEY_LEN: usize> = Arc<[u8; KEY_LEN]>;

pub(crate) fn reordered<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>>(
    key: &[u8; KEY_LEN],
) -> [u8; KEY_LEN] {
    let mut new_key = [0; KEY_LEN];
    for i in 0..KEY_LEN {
        new_key[i] = key[O::reorder(i)];
    }
    new_key
}

pub trait KeyOrdering<const KEY_LEN: usize>: Copy + Clone + Debug {
    fn reorder(at_depth: usize) -> usize;
}

pub trait KeySegmentation<const KEY_LEN: usize>: Copy + Clone + Debug {
    fn segment(at_depth: usize) -> usize;
}

#[derive(Copy, Clone, Debug)]
pub struct IdentityOrder {}

#[derive(Copy, Clone, Debug)]
pub struct SingleSegmentation {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for IdentityOrder {
    fn reorder(depth: usize) -> usize {
        depth
    }
}

impl<const KEY_LEN: usize> KeySegmentation<KEY_LEN> for SingleSegmentation {
    fn segment(_depth: usize) -> usize {
        0
    }
}

trait HeadVariant<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>: Sized {
    /// Returns a path byte fragment or all possible branch options
    /// at the given depth.
    fn peek(&self, at_depth: usize) -> Peek;

    /// Return the child stored at the provided depth under the provided key.
    /// Will return `self` when the fragment matches the key at the depth.
    fn child(&self, at_depth: usize, key: u8) -> Head<KEY_LEN, O, S>;

    /// Returns the number of leafs in the subtree under this node.
    fn count(&self) -> u32;

    /// Returns the number of segments in the subtree under this node.
    fn count_segment(&self, at_depth: usize) -> u32;

    /// Returns the xored sum of all hashes of leafs
    //  in the subtree under this node.
    fn hash(&self) -> u128;

    fn with_start(&self, _new_start_depth: usize) -> Head<KEY_LEN, O, S> {
        panic!("`with_start` not supported by {}", type_name::<Self>());
    }

    fn insert(&mut self, _child: Head<KEY_LEN, O, S>) -> Head<KEY_LEN, O, S> {
        panic!("`insert` not supported by {}", type_name::<Self>());
    }

    fn reinsert(&mut self, _child: Head<KEY_LEN, O, S>) -> Head<KEY_LEN, O, S> {
        panic!("`reinsert` not supported by {}", type_name::<Self>());
    }

    fn grow(&self) -> Head<KEY_LEN, O, S> {
        panic!("`grow` not supported by {}", type_name::<Self>());
    }

    /// Stores the provided key in the node. This returns a new node
    /// which may or may not share structure with the provided node.
    fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Head<KEY_LEN, O, S>;

    /// Enumerate the infixes given the provided key-prefix and infix range.
    fn infixes<F>(&self, key: [u8;KEY_LEN], start_depth: usize, end_depth: usize, f: F)
    where F: FnMut([u8; KEY_LEN]);
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
}

#[repr(C)]
pub union Head<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    unknown: ManuallyDrop<Unknown>,
    empty: ManuallyDrop<Empty<KEY_LEN, O, S>>,
    branch4: ManuallyDrop<Branch4<KEY_LEN, O, S>>,
    branch8: ManuallyDrop<Branch8<KEY_LEN, O, S>>,
    branch16: ManuallyDrop<Branch16<KEY_LEN, O, S>>,
    branch32: ManuallyDrop<Branch32<KEY_LEN, O, S>>,
    branch64: ManuallyDrop<Branch64<KEY_LEN, O, S>>,
    branch128: ManuallyDrop<Branch128<KEY_LEN, O, S>>,
    branch256: ManuallyDrop<Branch256<KEY_LEN, O, S>>,
    leaf: ManuallyDrop<Leaf<KEY_LEN, O, S>>,
}

unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> ByteEntry for Head<KEY_LEN, O, S> {
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

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> fmt::Debug for Head<KEY_LEN, O, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        dispatch!(self, variant, variant.fmt(f))
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Clone for Head<KEY_LEN, O, S> {
    fn clone(&self) -> Self {
        dispatch!(
            self,
            variant,
            Head::from(ManuallyDrop::into_inner(variant.clone()))
        )
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Drop for Head<KEY_LEN, O, S> {
    fn drop(&mut self) {
        dispatch_mut!(self, variant, ManuallyDrop::drop(variant))
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Default for Head<KEY_LEN, O, S> {
    fn default() -> Self {
        Empty::new().into()
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Head<KEY_LEN, O, S> {
    fn count(&self) -> u32 {
        dispatch!(self, variant, variant.count())
    }

    fn count_segment(&self, at_depth: usize) -> u32 {
        dispatch!(self, variant, variant.count_segment(at_depth))
    }

    fn with_start(&self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
        dispatch!(self, variant, variant.with_start(new_start_depth))
    }

    fn peek(&self, at_depth: usize) -> Peek {
        dispatch!(self, variant, variant.peek(at_depth))
    }

    fn child(&self, at_depth: usize, key: u8) -> Self {
        dispatch!(self, variant, variant.child(at_depth, key))
    }

    fn insert(&mut self, child: Self) -> Self {
        dispatch_mut!(self, variant, variant.insert(child))
    }

    fn reinsert(&mut self, child: Self) -> Self {
        dispatch_mut!(self, variant, variant.reinsert(child))
    }

    fn grow(&self) -> Self {
        dispatch!(self, variant, variant.grow())
    }

    fn hash(&self) -> u128 {
        dispatch!(self, variant, variant.hash())
    }

    fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Self {
        dispatch_mut!(self, variant, variant.put(key))
    }

    fn infixes<F>(&self, key: [u8;KEY_LEN], start_depth: usize, end_depth: usize, f: F)
    where F: FnMut([u8; KEY_LEN])
    {
        dispatch!(self, variant, variant.infixes(key, start_depth, end_depth, f));
    }
}

#[derive(Debug, Clone)]
pub struct PACT<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    root: Head<KEY_LEN, O, S>,
}

impl<const KEY_LEN: usize, O, S> PACT<KEY_LEN, O, S>
where
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
    [Head<KEY_LEN, O, S>; KEY_LEN]: Sized,
{

    pub fn new() -> Self {
        PACT {
            root: Empty::new().into(),
        }
    }

    pub fn put(&mut self, key: &SharedKey<KEY_LEN>) {
        self.root = self.root.put(key);
    }

    pub fn len(&self) -> u32 {
        self.root.count()
    }

    pub fn infixes<F>(&self, key: [u8;KEY_LEN], start_depth: usize, end_depth: usize, f: F)
    where
        F: FnMut([u8; KEY_LEN])
    {
        self.root.infixes(key, start_depth, end_depth, f);
    }
}

// Helpers
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

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::mem;

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head<64, IdentityOrder, SingleSegmentation>>(), 16);
    }

    #[test]
    fn empty_tree() {
        init();

        let _tree = PACT::<64, IdentityOrder, SingleSegmentation>::new();
    }

    #[test]
    fn tree_put_one() {
        init();

        const KEY_SIZE: usize = 64;
        let mut tree = PACT::<KEY_SIZE, IdentityOrder, SingleSegmentation>::new();
        let key = Arc::new([0; KEY_SIZE]);
        tree.put(&key);
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable4<Head<64, IdentityOrder, SingleSegmentation>>>(), 64);
        assert_eq!(mem::size_of::<BranchBody4<64, IdentityOrder, SingleSegmentation>>(), 64 * 3);
        assert_eq!(mem::size_of::<BranchBody8<64, IdentityOrder, SingleSegmentation>>(), 64 * 4);
        assert_eq!(mem::size_of::<BranchBody16<64, IdentityOrder, SingleSegmentation>>(), 64 * 6);
        assert_eq!(mem::size_of::<BranchBody32<64, IdentityOrder, SingleSegmentation>>(), 64 * 10);
        assert_eq!(mem::size_of::<BranchBody64<64, IdentityOrder, SingleSegmentation>>(), 64 * 18);
        assert_eq!(mem::size_of::<BranchBody128<64, IdentityOrder, SingleSegmentation>>(), 64 * 34);
        assert_eq!(mem::size_of::<BranchBody256<64, IdentityOrder, SingleSegmentation>>(), 64 * 66);
    }

    proptest! {
            #[test]
            fn tree_put(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
                let mut tree = PACT::<64, IdentityOrder, SingleSegmentation>::new();
                for entry in entries {
                    let mut key = [0; 64];
                    key.iter_mut().set_from(entry.iter().cloned());
                    tree.put(&Arc::new(key));
                }
            }

            #[test]
            fn tree_len(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
                let mut tree = PACT::<64, IdentityOrder, SingleSegmentation>::new();
                let mut set = HashSet::new();
                for entry in entries {
                    let mut key = [0; 64];
                    key.iter_mut().set_from(entry.iter().cloned());
                    tree.put(&Arc::new(key));
                    set.insert(key);
                }
                prop_assert_eq!(set.len() as u32, tree.len())
            }
    /*
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
    */
        }
}
