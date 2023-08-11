//Persitent Adaptive Cuckoo Trie
// Should be renamed to PATCH
// Persistent Adaptive Trie with Compression and Hashing

mod branch;
mod bytecursor;
mod leaf;
mod paddingcursor;
mod setops;

use branch::*;
use leaf::*;

use crate::bitset::ByteBitset;
use crate::bytetable;
use crate::bytetable::*;
use core::hash::Hasher;
use rand::thread_rng;
use rand::RngCore;
use siphasher::sip128::{Hasher128, SipHasher24};
use std::cmp::min;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::transmute;
use std::sync::Once;
use triomphe::Arc;

static mut SIP_KEY: [u8; 16] = [0; 16];
static INIT: Once = Once::new();

const HEAD_FRAGMENT_LEN: usize = 1;
const LEAF_FRAGMENT_LEN: usize = 1;

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

pub trait KeyOrdering<const KEY_LEN: usize>: Copy + Clone + Debug {
    fn tree_index(key_index: usize) -> usize;
    fn key_index(tree_index: usize) -> usize;

    fn tree_ordered(key: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
        let mut new_key = [0; KEY_LEN];
        for i in 0..KEY_LEN {
            new_key[i] = key[Self::key_index(i)];
        }
        new_key
    }

    fn key_ordered(tree_key: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
        let mut new_key = [0; KEY_LEN];
        for i in 0..KEY_LEN {
            new_key[i] = tree_key[Self::tree_index(i)];
        }
        new_key
    }
}

pub trait KeySegmentation<const KEY_LEN: usize>: Copy + Clone + Debug {
    fn segment(at_depth: usize) -> usize;
}

#[derive(Copy, Clone, Debug)]
pub struct IdentityOrder {}

#[derive(Copy, Clone, Debug)]
pub struct SingleSegmentation {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for IdentityOrder {
    fn key_index(tree_index: usize) -> usize {
        tree_index
    }
    fn tree_index(key_index: usize) -> usize {
        key_index
    }
}

impl<const KEY_LEN: usize> KeySegmentation<KEY_LEN> for SingleSegmentation {
    fn segment(_depth: usize) -> usize {
        0
    }
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
pub struct Head<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    tptr: u64,
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<S>,
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Head<KEY_LEN, O, S>
{
    pub fn empty() -> Self {
        Self {
            tptr: 0,
            key_ordering: PhantomData,
            key_segments: PhantomData,
        }
    }

    pub unsafe fn new<T>(tag: HeadTag, key: u8, ptr: *mut T) -> Self {
        Self {
            tptr: (ptr as u64 & 0x00_00_ff_ff_ff_ff_ff_ffu64)
                | ((key as u64) << 48)
                | ((tag as u64) << 56),
            key_ordering: PhantomData,
            key_segments: PhantomData,
        }
    }

    pub fn tag(&self) -> HeadTag {
        unsafe { transmute((self.tptr >> 56) as u8) }
    }

    pub fn key(&self) -> Option<u8> {
        if self.tag() == HeadTag::Empty {
            None
        } else {
            Some((self.tptr >> 48) as u8)
        }
    }

    pub fn set_key(&mut self, key: u8) {
        self.tptr = (self.tptr & 0xff_00_ff_ff_ff_ff_ff_ffu64) | ((key as u64) << 48);
    }

    pub unsafe fn ptr<T>(&self) -> *mut T {
        (((self.tptr << 16 as usize) as isize) >> 16 as usize) as *mut T
    }

    // Node
    fn count(&self) -> u64 {
        unsafe {
            match self.tag() {
                HeadTag::Empty => 0,
                HeadTag::Leaf => 1,
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::count(self.ptr()),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::count(self.ptr()),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::count(self.ptr()),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::count(self.ptr()),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::count(self.ptr()),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::count(self.ptr()),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::count(self.ptr()),
            }
        }
    }

    fn count_segment(&self, at_depth: usize) -> u64 {
        unsafe {
            match self.tag() {
                HeadTag::Empty => 0,
                HeadTag::Leaf => 1,
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::count_segment(self.ptr(), at_depth),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::count_segment(self.ptr(), at_depth),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::count_segment(self.ptr(), at_depth),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::count_segment(self.ptr(), at_depth),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::count_segment(self.ptr(), at_depth),
                HeadTag::Branch128 => {
                    Branch128::<KEY_LEN, O, S>::count_segment(self.ptr(), at_depth)
                }
                HeadTag::Branch256 => {
                    Branch256::<KEY_LEN, O, S>::count_segment(self.ptr(), at_depth)
                }
            }
        }
    }

    fn with_start(&self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
        unsafe {
            match self.tag() {
                HeadTag::Empty => Self::empty(),
                _ => {
                    if let Peek::Fragment(key) = self.peek(new_start_depth) {
                        let mut clone = self.clone();
                        clone.set_key(key);
                        clone
                    } else {
                        panic!("bad new_start_depth!");
                    }
                }
            }
        }
    }

    fn peek(&self, at_depth: usize) -> Peek {
        unsafe {
            match self.tag() {
                HeadTag::Empty => Peek::Branch(ByteBitset::new_empty()),
                HeadTag::Leaf => Peek::Fragment(Leaf::peek::<O>(self.ptr(), at_depth)),
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::peek(self.ptr(), at_depth),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::peek(self.ptr(), at_depth),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::peek(self.ptr(), at_depth),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::peek(self.ptr(), at_depth),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::peek(self.ptr(), at_depth),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::peek(self.ptr(), at_depth),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::peek(self.ptr(), at_depth),
            }
        }
    }

    fn branch(&self, key: u8) -> Self {
        unsafe {
            match self.tag() {
                HeadTag::Empty => panic!("no branch on empty"),
                HeadTag::Leaf => panic!("no branch on leaf"),
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::branch(self.ptr(), key),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::branch(self.ptr(), key),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::branch(self.ptr(), key),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::branch(self.ptr(), key),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::branch(self.ptr(), key),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::branch(self.ptr(), key),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::branch(self.ptr(), key),
            }
        }
    }

    fn child(&self, at_depth: usize, key: u8) -> Self {
        match self.peek(at_depth) {
            Peek::Fragment(byte) if byte == key => self.clone(),
            Peek::Branch(children) if children.is_set(key) => self.branch(key),
            _ => Head::empty(),
        }
    }

    fn insert(&mut self, child: Self) -> Self {
        dispatch_mut!(self, variant, variant.insert(child))
    }

    fn reinsert(&mut self, child: Self) -> Self {
        dispatch_mut!(self, variant, variant.reinsert(child))
    }

    fn grow(&self) -> Self {
        let key = unsafe { self.unknown.key };
        dispatch!(self, variant, variant.grow(key))
    }

    fn hash(&self) -> u128 {
        dispatch!(self, variant, variant.hash())
    }

    unsafe fn min(&self) -> *const Leaf<KEY_LEN> {
        unsafe {
            match self.tag() {
                HeadTag::Empty => std::ptr::null_mut(),
                HeadTag::Leaf => self.ptr::<Leaf<KEY_LEN>>(),
                HeadTag::Branch4 => (*self.ptr::<Branch4<KEY_LEN, O, S>>()).min,
                HeadTag::Branch8 => (*self.ptr::<Branch8<KEY_LEN, O, S>>()).min,
                HeadTag::Branch16 => (*self.ptr::<Branch16<KEY_LEN, O, S>>()).min,
                HeadTag::Branch32 => (*self.ptr::<Branch32<KEY_LEN, O, S>>()).min,
                HeadTag::Branch64 => (*self.ptr::<Branch64<KEY_LEN, O, S>>()).min,
                HeadTag::Branch128 => (*self.ptr::<Branch128<KEY_LEN, O, S>>()).min,
                HeadTag::Branch256 => (*self.ptr::<Branch256<KEY_LEN, O, S>>()).min,
            }
        }
    }

    fn put(&mut self, key: &SharedKey<KEY_LEN>, start_depth: usize) -> Self {
        dispatch_mut!(self, variant, variant.put(key, start_depth))
    }

    fn infixes<const INFIX_LEN: usize, F>(
        &self,
        key: [u8; KEY_LEN],
        depth: usize,
        start_depth: usize,
        end_depth: usize,
        f: F,
        out: &mut Vec<[u8; INFIX_LEN]>,
    ) where
        F: Copy + Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN],
    {
        dispatch!(
            self,
            variant,
            variant.infixes(key, depth, start_depth, end_depth, f, out)
        );
    }

    fn has_prefix(&self, depth: usize, key: [u8; KEY_LEN], end_depth: usize) -> bool {
        dispatch!(self, variant, variant.has_prefix(depth, key, end_depth))
    }

    fn segmented_len(&self, depth: usize, key: [u8; KEY_LEN], start_depth: usize) -> usize {
        dispatch!(
            self,
            variant,
            variant.segmented_len(depth, key, start_depth)
        )
    }
}

unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> ByteEntry
    for Head<KEY_LEN, O, S>
{
    fn zeroed() -> Self {
        Self::empty()
    }

    fn key(&self) -> Option<u8> {
        unsafe {
            if self.tag() == HeadTag::Empty {
                None
            } else {
                Some(self.key())
            }
        }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> fmt::Debug
    for Head<KEY_LEN, O, S>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        dispatch!(self, variant, variant.fmt(f))
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Clone
    for Head<KEY_LEN, O, S>
{
    fn clone(&self) -> Self {
        dispatch!(
            self,
            variant,
            Head::from(ManuallyDrop::into_inner(variant.clone()))
        )
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Drop
    for Head<KEY_LEN, O, S>
{
    fn drop(&mut self) {
        unsafe {
            match self.tag() {
                HeadTag::Empty => return,
                HeadTag::Leaf => Leaf::<KEY_LEN>::rc_dec(self.ptr()),
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::rc_dec(self.ptr()),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::rc_dec(self.ptr()),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::rc_dec(self.ptr()),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::rc_dec(self.ptr()),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::rc_dec(self.ptr()),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::rc_dec(self.ptr()),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::rc_dec(self.ptr()),
            }
        }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Default
    for Head<KEY_LEN, O, S>
{
    fn default() -> Self {
        Empty::new().into()
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
        self.root = self.root.put(key, 0);
    }

    pub fn len(&self) -> u32 {
        self.root.count()
    }

    pub fn infixes<const INFIX_LEN: usize, F>(
        &self,
        key: [u8; KEY_LEN],
        start_depth: usize,
        end_depth: usize,
        f: F,
    ) -> Vec<[u8; INFIX_LEN]>
    where
        F: Copy + Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN],
    {
        let mut out = vec![];
        self.root.infixes(
            O::tree_ordered(&key),
            0,
            O::tree_index(start_depth),
            O::tree_index(end_depth),
            f,
            &mut out,
        );
        out
    }

    pub fn has_prefix(&self, key: [u8; KEY_LEN], end_depth: usize) -> bool {
        self.root
            .has_prefix(0, O::tree_ordered(&key), O::tree_index(end_depth))
    }

    pub fn segmented_len(&self, key: [u8; KEY_LEN], start_depth: usize) -> usize {
        self.root
            .segmented_len(0, O::tree_ordered(&key), O::tree_index(start_depth))
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
    use std::iter::FromIterator;
    use std::mem;

    #[test]
    fn head_size() {
        assert_eq!(
            mem::size_of::<Head<64, IdentityOrder, SingleSegmentation>>(),
            16
        );
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
        assert_eq!(
            mem::size_of::<ByteTable4<Head<64, IdentityOrder, SingleSegmentation>>>(),
            64
        );
        assert_eq!(
            mem::size_of::<Branch4<64, IdentityOrder, SingleSegmentation>>(),
            64 * 3
        );
        assert_eq!(
            mem::size_of::<Branch8<64, IdentityOrder, SingleSegmentation>>(),
            64 * 4
        );
        assert_eq!(
            mem::size_of::<Branch16<64, IdentityOrder, SingleSegmentation>>(),
            64 * 6
        );
        assert_eq!(
            mem::size_of::<Branch32<64, IdentityOrder, SingleSegmentation>>(),
            64 * 10
        );
        assert_eq!(
            mem::size_of::<Branch64<64, IdentityOrder, SingleSegmentation>>(),
            64 * 18
        );
        assert_eq!(
            mem::size_of::<Branch128<64, IdentityOrder, SingleSegmentation>>(),
            64 * 34
        );
        assert_eq!(
            mem::size_of::<Branch256<64, IdentityOrder, SingleSegmentation>>(),
            64 * 66
        );
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

        #[test]
        fn tree_infixes(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64, IdentityOrder, SingleSegmentation>::new();
            let mut set = HashSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(&Arc::new(key));
                set.insert(key);
            }
            let mut set_vec = Vec::from_iter(set.into_iter());
            let mut tree_vec = tree.infixes([0; 64], 0, 63, |x| x);

            set_vec.sort();
            tree_vec.sort();

            prop_assert_eq!(set_vec, tree_vec);
        }
    }
}
