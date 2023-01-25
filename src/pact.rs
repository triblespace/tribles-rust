mod branch;
mod empty;
mod leaf;
mod macros;
mod path;

use branch::*;
use empty::*;
use leaf::*;
use macros::*;
use path::*;

use crate::bitset::ByteBitset;
use crate::bytetable;
use crate::bytetable::*;
use crate::query::ByteCursor; //CursorIterator
use rand::thread_rng;
use rand::RngCore;
use std::cmp::{max, min};
use std::mem;
use std::sync::Arc;
use std::sync::Once;
use core::hash::Hasher;
use siphasher::sip128::{Hasher128, SipHasher24};
use std::fmt;
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::mem::{transmute, MaybeUninit};

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

const HEAD_SIZE: usize = 16;
const HEAD_FRAGMENT_LEN: usize = 5;
const LEAF_FRAGMENT_LEN: usize = 14;

fn index_start(infix_start: usize, index: usize) -> usize {
    index - infix_start
}

fn index_end(infix_len: usize, infix_end: usize, index: usize) -> usize {
    (index + infix_len) - infix_end
}

fn copy_end(target: &mut [u8], source: &[u8], end_index: usize) {
    let target_len = target.len();
    let used_len = min(end_index as usize, target_len);
    let target_range = &mut target[target_len - used_len..];
    let source_range = &source[end_index - used_len..end_index];
    target_range.copy_from_slice(source_range);
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
    Path14,
    Path30,
    Path46,
    Path62,
    Leaf,
}

#[repr(C)]
pub union Head<const KEY_LEN: usize> {
    unknown: ManuallyDrop<Unknown>,
    empty: ManuallyDrop<Empty>,
    branch4: ManuallyDrop<Branch4<KEY_LEN>>,
    branch8: ManuallyDrop<Branch8<KEY_LEN>>,
    branch16: ManuallyDrop<Branch16<KEY_LEN>>,
    branch32: ManuallyDrop<Branch32<KEY_LEN>>,
    branch64: ManuallyDrop<Branch64<KEY_LEN>>,
    branch128: ManuallyDrop<Branch128<KEY_LEN>>,
    branch256: ManuallyDrop<Branch256<KEY_LEN>>,
    path14: ManuallyDrop<Path14<KEY_LEN>>,
    path30: ManuallyDrop<Path30<KEY_LEN>>,
    path46: ManuallyDrop<Path46<KEY_LEN>>,
    path62: ManuallyDrop<Path62<KEY_LEN>>,
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
    fn wrap_path(&self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let expanded = self.with_start_depth(start_depth, key);

        let actual_start_depth = expanded.start_depth() as usize;
        if start_depth == actual_start_depth {
            return expanded;
        }

        let path_length = actual_start_depth - start_depth;

        if path_length <= 14 + HEAD_FRAGMENT_LEN {
            return Path14::new(start_depth, &key, expanded).into();
        }

        if path_length <= 30 + HEAD_FRAGMENT_LEN {
            return Path30::new(start_depth, &key, expanded).into();
        }

        if path_length <= 46 + HEAD_FRAGMENT_LEN {
            return Path46::new(start_depth, &key, expanded).into();
        }

        if path_length <= 62 + HEAD_FRAGMENT_LEN {
            return Path62::new(start_depth, &key, expanded).into();
        }

        panic!("Fragment too long for path to hold.");
    }

    fn start_depth(&self) -> u8 {
        unsafe {
            if self.unknown.tag == HeadTag::Empty {
                panic!("Called `start_depth` on `Empty`.");
            }
            self.unknown.start_depth
        }
    }

    fn count(&self) -> u64 {
        dispatch!(self, variant, variant.count())
    }

    fn with_start_depth(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        dispatch!(
            self,
            variant,
            variant.with_start_depth(new_start_depth, key)
        )
    }

    fn peek(&self, at_depth: usize) -> Option<u8> {
        dispatch!(self, variant, variant.peek(at_depth))
    }

    fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
        dispatch!(self, variant, variant.propose(at_depth, result_set))
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

impl<'a, const KEY_LEN: usize> PACT<KEY_LEN>
where
    [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1]: Sized,
{
    const KEY_LEN_CHECK: usize = KEY_LEN - 64;

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

pub struct PACTCursor<'a, const KEY_LEN: usize>
where
    [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1]: Sized,
{
    depth: usize,
    path: [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1],
}

impl<'a, const KEY_LEN: usize> PACTCursor<'a, KEY_LEN>
where
    [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1]: Sized,
{
    pub fn new(tree: &'a PACT<KEY_LEN>) -> Self {
        let mut new = Self {
            depth: 0,
            path: [None; KEY_LEN + 1],
        };
        new.path[0] = Some(&tree.root);
        return new;
    }
}

impl<'a, const KEY_LEN: usize> ByteCursor for PACTCursor<'a, KEY_LEN>
where
    [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1]: Sized,
{
    fn peek(&self) -> Option<u8> {
        return self.path[self.depth]
            .expect("peeked path should exist")
            .peek(self.depth);
    }

    fn propose(&self, bitset: &mut ByteBitset) {
        self.path[self.depth]
            .expect("proposed path should exist")
            .propose(self.depth, bitset);
    }

    fn pop(&mut self) {
        self.depth -= 1;
    }

    fn push(&mut self, byte: u8) {
        self.path[self.depth + 1] = self.path[self.depth];
        //.expect("pushed path should exist")
        //.get(self.depth, byte);
        self.depth += 1;
    }

    fn segment_count(&self) -> u32 {
        return 0;
        //return self.path[self.depth].segment_count(self.depth);
    }
}

/*
    pub fn iterate(self: Cursor) CursorIterator(Cursor, key_length) {
        return CursorIterator(Cursor, key_length).init(self);
    }
*/

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
        assert_eq!(mem::size_of::<BranchBody4<64>>(), 64 * 2);
        assert_eq!(mem::size_of::<BranchBody8<64>>(), 64 * 3);
        assert_eq!(mem::size_of::<BranchBody16<64>>(), 64 * 5);
        assert_eq!(mem::size_of::<BranchBody32<64>>(), 64 * 9);
        assert_eq!(mem::size_of::<BranchBody64<64>>(), 64 * 17);
        assert_eq!(mem::size_of::<BranchBody128<64>>(), 64 * 33);
        assert_eq!(mem::size_of::<BranchBody256<64>>(), 64 * 65);
    }

    #[test]
    fn fragment_size() {
        assert_eq!(mem::size_of::<PathBody14<64>>(), 16 * 2);
        assert_eq!(mem::size_of::<PathBody30<64>>(), 16 * 3);
        assert_eq!(mem::size_of::<PathBody46<64>>(), 16 * 4);
        assert_eq!(mem::size_of::<PathBody62<64>>(), 16 * 5);
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
            //let entry_set = HashSet::from_iter(entries.iter().cloned());
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
    }
}
