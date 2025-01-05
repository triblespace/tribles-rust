//! # Persistent Adaptive Trie with Cuckoo-compression and Hash-maintenance
//!
//! The PATCH is a novel adaptive trie, that uses cuckoo hashing
//! as a node compression technique to store between 2 and 256
//! children wide nodes with a single node type.
//! It further uses efficient hash maintenance to provide fast
//! set operations over these tries.
//!
#![allow(unstable_name_collisions)]

mod branch;
mod bytetable;
mod entry;
mod leaf;

use sptr::Strict;

use branch::*;
pub use entry::Entry;
use leaf::*;

use bytetable::*;
use rand::thread_rng;
use rand::RngCore;
use std::cmp::Reverse;
use std::convert::TryInto;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::transmute;
use std::ptr::NonNull;
use std::sync::Once;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("PATCH tagged pointers require 64-bit targets");

static mut SIP_KEY: [u8; 16] = [0; 16];
static INIT: Once = Once::new();

/// Initializes the SIP key used for key hashing.
/// This function is called automatically when a new PATCH is created.
fn init_sip_key() {
    INIT.call_once(|| {
        bytetable::init();

        let mut rng = thread_rng();
        unsafe {
            rng.fill_bytes(&mut SIP_KEY[..]);
        }
    });
}

/// A trait is used to provide a re-ordered view of the keys stored in the PATCH.
/// This allows for different PATCH instances share the same leaf nodes,
/// independent of the key ordering used in the tree.
pub trait KeyOrdering<const KEY_LEN: usize>: Copy + Clone + Debug {
    /// Returns the index in the tree view, given the index in the key view.
    ///
    /// This is the inverse of [Self::key_index].
    fn tree_index(key_index: usize) -> usize;

    /// Returns the index in the key view, given the index in the tree view.
    ///
    /// This is the inverse of [Self::tree_index].
    fn key_index(tree_index: usize) -> usize;

    /// Reorders the key from the shared key ordering to the tree ordering.
    fn tree_ordered(key: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
        let mut new_key = [0; KEY_LEN];
        for i in 0..KEY_LEN {
            new_key[i] = key[Self::key_index(i)];
        }
        new_key
    }

    /// Reorders the key from the tree ordering to the shared key ordering.
    fn key_ordered(tree_key: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
        let mut new_key = [0; KEY_LEN];
        for i in 0..KEY_LEN {
            new_key[i] = tree_key[Self::tree_index(i)];
        }
        new_key
    }
}

/// This trait is used to segment keys stored in the PATCH.
/// The segmentation is used to determine sub-fields of the key,
/// allowing for segment based operations, like counting the number
/// of elements in a segment with a given prefix without traversing the tree.
///
/// Note that the segmentation is defined on the shared key ordering,
/// and should thus be only implemented once, independent of additional key orderings.
///
/// See [TribleSegmentation](crate::trible::TribleSegmentation) for an example that segments keys into entity,
/// attribute, and value segments.
pub trait KeySegmentation<const KEY_LEN: usize>: Copy + Clone + Debug {
    /// Returns the segment index for the given key index.
    fn segment(key_index: usize) -> usize;
}

/// A `KeyOrdering` that does not reorder the keys.
/// This is useful for keys that are already ordered in the desired way.
/// This is the default ordering.
#[derive(Copy, Clone, Debug)]
pub struct IdentityOrder {}

/// A `KeySegmentation` that does not segment the keys.
/// This is useful for keys that do not have a segment structure.
/// This is the default segmentation.
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
pub(crate) enum HeadTag {
    Leaf = 1,
    Branch2 = 2,
    Branch4 = 3,
    Branch8 = 4,
    Branch16 = 5,
    Branch32 = 6,
    Branch64 = 7,
    Branch128 = 8,
    Branch256 = 9,
}

#[derive(Debug)]
pub(crate) enum BodyRef<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    Leaf(&'a Leaf<KEY_LEN>),
    Branch(&'a BranchN<KEY_LEN, O, S>),
}

#[derive(Debug)]
pub(crate) enum BodyMut<
    'a,
    const KEY_LEN: usize,
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
> {
    Leaf(&'a Leaf<KEY_LEN>),
    Branch(&'a mut BranchN<KEY_LEN, O, S>),
}

pub(crate) trait Body {
    const TAG: HeadTag;
}

#[repr(C)]
pub(crate) struct Head<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    tptr: std::ptr::NonNull<u8>,
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<S>,
}

unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Send
    for Head<KEY_LEN, O, S>
{
}
unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Sync
    for Head<KEY_LEN, O, S>
{
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Head<KEY_LEN, O, S>
{
    pub(crate) fn new<T: Body>(key: u8, body: NonNull<T>) -> Self {
        unsafe {
            Self {
                tptr: std::ptr::NonNull::new_unchecked((body.as_ptr()).map_addr(|addr| {
                    ((addr as u64 & 0x00_00_ff_ff_ff_ff_ff_ffu64)
                        | ((key as u64) << 48)
                        | ((T::TAG as u64) << 56)) as usize
                }) as *mut u8),
                key_ordering: PhantomData,
                key_segments: PhantomData,
            }
        }
    }

    #[inline]
    pub(crate) fn tag(&self) -> HeadTag {
        unsafe { transmute((self.tptr.as_ptr() as u64 >> 56) as u8) }
    }

    #[inline]
    pub(crate) fn key(&self) -> u8 {
        (self.tptr.as_ptr() as u64 >> 48) as u8
    }

    #[inline]
    pub(crate) fn set_key(&mut self, key: u8) {
        self.tptr = std::ptr::NonNull::new(self.tptr.as_ptr().map_addr(|addr| {
            ((addr as u64 & 0xff_00_ff_ff_ff_ff_ff_ffu64) | ((key as u64) << 48)) as usize
        }))
        .unwrap();
    }

    #[inline]
    unsafe fn ptr<T: Body>(&self) -> NonNull<T> {
        debug_assert_eq!(T::TAG, self.tag());

        NonNull::new_unchecked(
            self.tptr
                .as_ptr()
                .map_addr(|addr| ((((addr as u64) << 16) as i64) >> 16) as usize)
                as *mut T,
        )
    }

    #[inline]
    pub(crate) fn set_body<T: Body>(&mut self, ptr: NonNull<T>) {
        unsafe {
            self.tptr = NonNull::new_unchecked(ptr.as_ptr().map_addr(|addr| {
                ((addr as u64 & 0x00_00_ff_ff_ff_ff_ff_ffu64)
                    | (self.tptr.as_ptr() as u64 & 0x00_ff_00_00_00_00_00_00u64)
                    | ((T::TAG as u64) << 56)) as usize
            }) as *mut u8)
        }
    }

    pub(crate) fn with_start(mut self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
        let leaf_key = self.leaf_key();
        let i = O::key_index(new_start_depth);
        let key = leaf_key[i];
        self.set_key(key);
        self
    }

    pub(crate) fn body<'a>(&'a self) -> BodyRef<'a, KEY_LEN, O, S> {
        unsafe {
            match self.tag() {
                HeadTag::Leaf => BodyRef::Leaf(self.ptr().as_ref()),
                HeadTag::Branch2 => BodyRef::Branch(
                    self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>>()
                        .as_ref(),
                ),
                HeadTag::Branch4 => BodyRef::Branch(
                    self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>>()
                        .as_ref(),
                ),
                HeadTag::Branch8 => BodyRef::Branch(
                    self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>>()
                        .as_ref(),
                ),
                HeadTag::Branch16 => BodyRef::Branch(
                    self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>>()
                        .as_ref(),
                ),
                HeadTag::Branch32 => BodyRef::Branch(
                    self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>>()
                        .as_ref(),
                ),
                HeadTag::Branch64 => BodyRef::Branch(
                    self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>>()
                        .as_ref(),
                ),
                HeadTag::Branch128 => BodyRef::Branch(
                    self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>>()
                        .as_ref(),
                ),
                HeadTag::Branch256 => BodyRef::Branch(
                    self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 256]>>()
                        .as_ref(),
                ),
            }
        }
    }

    pub(crate) fn body_mut<'a>(&'a mut self) -> BodyMut<'a, KEY_LEN, O, S> {
        unsafe {
            match self.tag() {
                HeadTag::Leaf => BodyMut::Leaf(self.ptr::<Leaf<KEY_LEN>>().as_ref()),
                HeadTag::Branch2 => {
                    let mut branch: NonNull<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>,
                    > = self.ptr();
                    if let Some(mut copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyMut::Branch(copy.as_mut())
                    } else {
                        BodyMut::Branch(branch.as_mut())
                    }
                }
                HeadTag::Branch4 => {
                    let mut branch: NonNull<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>,
                    > = self.ptr();
                    if let Some(mut copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyMut::Branch(copy.as_mut())
                    } else {
                        BodyMut::Branch(branch.as_mut())
                    }
                }
                HeadTag::Branch8 => {
                    let mut branch: NonNull<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>,
                    > = self.ptr();
                    if let Some(mut copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyMut::Branch(copy.as_mut())
                    } else {
                        BodyMut::Branch(branch.as_mut())
                    }
                }
                HeadTag::Branch16 => {
                    let mut branch: NonNull<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>,
                    > = self.ptr();
                    if let Some(mut copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyMut::Branch(copy.as_mut())
                    } else {
                        BodyMut::Branch(branch.as_mut())
                    }
                }
                HeadTag::Branch32 => {
                    let mut branch: NonNull<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>,
                    > = self.ptr();
                    if let Some(mut copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyMut::Branch(copy.as_mut())
                    } else {
                        BodyMut::Branch(branch.as_mut())
                    }
                }
                HeadTag::Branch64 => {
                    let mut branch: NonNull<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>,
                    > = self.ptr();
                    if let Some(mut copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyMut::Branch(copy.as_mut())
                    } else {
                        BodyMut::Branch(branch.as_mut())
                    }
                }
                HeadTag::Branch128 => {
                    let mut branch: NonNull<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>,
                    > = self.ptr();
                    if let Some(mut copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyMut::Branch(copy.as_mut())
                    } else {
                        BodyMut::Branch(branch.as_mut())
                    }
                }
                HeadTag::Branch256 => {
                    let mut branch: NonNull<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 256]>,
                    > = self.ptr();
                    if let Some(mut copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyMut::Branch(copy.as_mut())
                    } else {
                        BodyMut::Branch(branch.as_mut())
                    }
                }
            }
        }
    }

    pub fn upsert<F>(&mut self, inserted: Head<KEY_LEN, O, S>, update: F)
    where
        F: FnOnce(&mut Head<KEY_LEN, O, S>, Head<KEY_LEN, O, S>),
    {
        match self.body_mut() {
            BodyMut::Leaf(_) => panic!("upsert on leaf"),
            BodyMut::Branch(branch) => {
                let inserted = inserted.with_start(branch.end_depth as usize);
                let key = inserted.key();
                if let Some(child) = branch.child_table.table_get_mut(key) {
                    let old_child_hash = child.hash();
                    let old_child_segment_count = child.count_segment(branch.end_depth as usize);
                    let old_child_leaf_count = child.count();

                    update(child, inserted);

                    branch.hash = (branch.hash ^ old_child_hash) ^ child.hash();
                    branch.segment_count = (branch.segment_count - old_child_segment_count)
                        + child.count_segment(branch.end_depth as usize);
                    branch.leaf_count = (branch.leaf_count - old_child_leaf_count) + child.count();
                } else {
                    let end_depth = branch.end_depth as usize;
                    branch.leaf_count += inserted.count();
                    branch.segment_count += inserted.count_segment(end_depth);
                    branch.hash ^= inserted.hash();

                    let mut branch: *mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]> = branch;
                    let mut inserted = inserted;
                    unsafe {
                        if (*branch).child_table.len() == 2 {
                            let Some(displaced) = (*branch).child_table.table_insert(inserted) else {
                                return;
                            };
                            inserted = displaced;
                            branch = Branch2::<KEY_LEN, O, S>::grow(NonNull::new_unchecked(branch as _)).as_ptr();
                        }
                        if (*branch).child_table.len() == 4 {
                            let Some(displaced) = (*branch).child_table.table_insert(inserted) else {
                                self.set_body(NonNull::new_unchecked(branch as *mut Branch4::<KEY_LEN, O, S>));
                                return;
                            };
                            inserted = displaced;
                            branch = Branch4::<KEY_LEN, O, S>::grow(NonNull::new_unchecked(branch as _)).as_ptr();
                        }
                        if (*branch).child_table.len() == 8 {
                            let Some(displaced) = (*branch).child_table.table_insert(inserted) else {
                                self.set_body(NonNull::new_unchecked(branch as *mut Branch8::<KEY_LEN, O, S>));
                                return;
                            };
                            inserted = displaced;
                            branch = Branch8::<KEY_LEN, O, S>::grow(NonNull::new_unchecked(branch as _)).as_ptr();
                        }
                        if (*branch).child_table.len() == 16 {
                            let Some(displaced) = (*branch).child_table.table_insert(inserted) else {
                                self.set_body(NonNull::new_unchecked(branch as *mut Branch16::<KEY_LEN, O, S>));
                                return;
                            };
                            inserted = displaced;
                            branch = Branch16::<KEY_LEN, O, S>::grow(NonNull::new_unchecked(branch as _)).as_ptr();
                        }
                        if (*branch).child_table.len() == 32 {
                            let Some(displaced) = (*branch).child_table.table_insert(inserted) else {
                                self.set_body(NonNull::new_unchecked(branch as *mut Branch32::<KEY_LEN, O, S>));
                                return;
                            };
                            inserted = displaced;
                            branch = Branch32::<KEY_LEN, O, S>::grow(NonNull::new_unchecked(branch as _)).as_ptr();
                        }
                        if (*branch).child_table.len() == 64 {
                            let Some(displaced) = (*branch).child_table.table_insert(inserted) else {
                                self.set_body(NonNull::new_unchecked(branch as *mut Branch64::<KEY_LEN, O, S>));
                                return;
                            };
                            inserted = displaced;
                            branch = Branch64::<KEY_LEN, O, S>::grow(NonNull::new_unchecked(branch as _)).as_ptr();
                        }
                        if (*branch).child_table.len() == 128 {
                            let Some(displaced) = (*branch).child_table.table_insert(inserted) else {
                                self.set_body(NonNull::new_unchecked(branch as *mut Branch128::<KEY_LEN, O, S>));
                                return;
                            };
                            inserted = displaced;
                            branch = Branch128::<KEY_LEN, O, S>::grow(NonNull::new_unchecked(branch as _)).as_ptr();
                        }
                        if (*branch).child_table.len() == 256 {
                            let Some(_) = (*branch).child_table.table_insert(inserted) else {
                                self.set_body(NonNull::new_unchecked(branch as *mut Branch256::<KEY_LEN, O, S>));
                                return;
                            };
                            panic!("failed to insert on Branch256");
                        }
    
                        panic!("failed to insert on non branch");
                    }
                }
            }
        }
    }

    pub(crate) fn count(&self) -> u64 {
        match self.body() {
            BodyRef::Leaf(_) => 1,
            BodyRef::Branch(branch) => (*branch).leaf_count,
        }
    }

    pub(crate) fn count_segment(&self, at_depth: usize) -> u64 {
        match self.body() {
            BodyRef::Leaf(_) => 1,
            BodyRef::Branch(branch) => {
                branch::Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::count_segment(
                    branch, at_depth,
                )
            }
        }
    }

    pub(crate) fn hash(&self) -> u128 {
            match self.body() {
                BodyRef::Leaf(leaf) => leaf.hash(),
                BodyRef::Branch(branch) => branch.hash,
            }
    }

    pub(crate) fn end_depth(&self) -> usize {
        match self.body() {
            BodyRef::Leaf(_) => KEY_LEN as usize,
            BodyRef::Branch(branch) => (*branch).end_depth as usize,
        }
    }

    pub(crate) fn childleaf(&self) -> *const Leaf<KEY_LEN> {
        match self.body() {
            BodyRef::Leaf(leaf) => leaf,
            BodyRef::Branch(branch) => (*branch).childleaf,
        }
    }

    pub(crate) fn leaf_key<'a>(&'a self) -> &'a [u8; KEY_LEN] {
        unsafe{ &(*self.childleaf()).key }
    }

    pub(crate) fn remove_leaf(
        slot: &mut Option<Self>,
        leaf_key: &[u8; KEY_LEN],
        start_depth: usize,
    ) {
        if let Some(this) = slot {
            let head_key = this.leaf_key();

            let end_depth = std::cmp::min(this.end_depth(), KEY_LEN);
            for depth in start_depth..end_depth {
                let i = O::key_index(depth);
                if head_key[i] != leaf_key[i] {
                    return;
                }
            }
            match this.body_mut() {
                BodyMut::Leaf(_) => {
                    slot.take();
                }
                BodyMut::Branch(branch) => {
                    let key = leaf_key[end_depth];
                    let removed_leafchild = leaf_key == branch.childleaf_key();
                    if let Some(child_slot) = branch.child_table.table_get_slot(key) {
                        if let Some(child) = child_slot {
                            let old_child_hash = child.hash();
                            let old_child_segment_count =
                                child.count_segment(branch.end_depth as usize);
                            let old_child_leaf_count = child.count();
                            
                            Self::remove_leaf(child_slot, leaf_key, end_depth);
                            if let Some(child) = child_slot {
                                if removed_leafchild {
                                    branch.childleaf = child.childleaf();
                                }
                                branch.hash = (branch.hash ^ old_child_hash) ^ child.hash();
                                branch.segment_count = (branch.segment_count
                                    - old_child_segment_count)
                                    + child.count_segment(branch.end_depth as usize);
                                branch.leaf_count =
                                    (branch.leaf_count - old_child_leaf_count) + child.count();
                                // ^ Note that the leaf_count can never be <= 1 here, because we're in a branch
                                // at least one other child must exist.
                            } else {
                                branch.leaf_count = branch.leaf_count - old_child_leaf_count;
                                match branch.leaf_count {
                                    0 => {
                                        panic!("branch should have been collected previously")
                                    }
                                    1 => {
                                        for child in &mut branch.child_table {
                                            if let Some(child) = child.take() {
                                                slot.replace(child.with_start(start_depth));
                                                return;
                                            }
                                        }
                                    }
                                    _ => {
                                        if removed_leafchild {
                                            let child = branch.child_table.iter().find_map(|s| s.as_ref()).expect("child should exist");
                                            branch.childleaf = child.childleaf();
                                        }
                                        branch.hash = branch.hash ^ old_child_hash;
                                        branch.segment_count =
                                            branch.segment_count - old_child_segment_count;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn insert_leaf(&mut self, leaf: Self, start_depth: usize) {
        let head_key = self.leaf_key();
        let leaf_key = leaf.leaf_key();

        let end_depth = std::cmp::min(self.end_depth(), KEY_LEN);
        for depth in start_depth..end_depth {
            let i = O::key_index(depth);
            if head_key[i] != leaf_key[i] {
                let new_head = Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>::new(
                    self.key(),
                    depth,
                    leaf.with_start(depth),
                );

                let old_head = std::mem::replace(self, new_head);

                self.upsert(old_head.with_start(depth), |_, _| unreachable!());
                return;
            }
        }

        if end_depth != KEY_LEN {
            self.upsert(leaf, |child, inserted| {
                child.insert_leaf(inserted, end_depth)
            });
        }
    }

    pub(crate) fn infixes<const PREFIX_LEN: usize, const INFIX_LEN: usize, F>(
        &self,
        prefix: &[u8; PREFIX_LEN],
        at_depth: usize,
        f: &mut F,
    ) where
        F: FnMut(&[u8; INFIX_LEN]),
    {
        match self.body() {
            BodyRef::Leaf(leaf) => Leaf::<KEY_LEN>::infixes::<PREFIX_LEN, INFIX_LEN, O, S, F>(
                leaf, prefix, at_depth, f,
            ),
            BodyRef::Branch(branch) => {
                Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::infixes(
                    branch, prefix, at_depth, f,
                )
            }
        }
    }

    pub(crate) fn has_prefix<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        match self.body() {
            BodyRef::Leaf(leaf) => {
                Leaf::<KEY_LEN>::has_prefix::<O, PREFIX_LEN>(leaf, at_depth, prefix)
            }
            BodyRef::Branch(branch) => {
                branch::Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::has_prefix(
                    branch, at_depth, prefix,
                )
            }
        }
    }

    pub(crate) fn segmented_len<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        match self.body() {
            BodyRef::Leaf(leaf) => {
                leaf::Leaf::<KEY_LEN>::segmented_len::<O, PREFIX_LEN>(leaf, at_depth, prefix)
            }
            BodyRef::Branch(branch) => {
                Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::segmented_len(
                    branch, at_depth, prefix,
                )
            }
        }
    }

    pub(crate) fn union(&mut self, mut other: Self, at_depth: usize) {
        let self_hash = self.hash();
        let other_hash = other.hash();
        if self_hash == other_hash {
            return;
        }
        let self_depth = self.end_depth();
        let other_depth = other.end_depth();

        let self_key = self.leaf_key();
        let other_key = other.leaf_key();
        for depth in at_depth..std::cmp::min(self_depth, other_depth) {
            let i = O::key_index(depth);
            if self_key[i] != other_key[i] {
                let new_head = Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>::new(
                    self.key(),
                    depth,
                    other.with_start(depth),
                );

                let old_self = std::mem::replace(self, new_head);

                self.upsert(old_self.with_start(depth), |_, _| unreachable!());
                return;
            }
        }

        if self_depth < other_depth {
            self.upsert(other, |child, inserted| child.union(inserted, self_depth));
            return;
        }

        if other_depth < self_depth {
            let new_self = other.with_start(at_depth);
            let old_self = std::mem::replace(self, new_self);
            self.upsert(old_self, |child, inserted| {
                child.union(inserted, other_depth)
            });
            return;
        }

        match other.body_mut() {
            // we already checked for equality by comparing the hashes,
            // if they are not equal, the keys must be different which is
            // already handled by the above code.
            BodyMut::Leaf(_) => unreachable!(),
            BodyMut::Branch(other_branch) => {
                for other_child in other_branch
                    .child_table
                    .iter_mut()
                    .filter_map(Option::take)
                {
                    self.upsert(other_child, |child, inserted| {
                        child.union(inserted, self_depth)
                    });
                }
            },
        }
    }

    pub(crate) fn intersect(&self, other: &Self, at_depth: usize) -> Option<Self> {
        let self_hash = self.hash();
        let other_hash = other.hash();
        if self_hash == other_hash {
            return Some(self.clone());
        }
        let self_depth = self.end_depth();
        let other_depth = other.end_depth();

        let self_key = self.leaf_key();
        let other_key = other.leaf_key();
        for depth in at_depth..std::cmp::min(self_depth, other_depth) {
            let i = O::key_index(depth);
            if self_key[i] != other_key[i] {
                return None;
            }
        }

        if self_depth < other_depth {
            // This means that there can be at most one child in self
            // that might intersect with other.
            match self.body() {
                BodyRef::Leaf(_) => unreachable!(),
                // If the depth of self is less than the depth of other, then it can't be a leaf.
                BodyRef::Branch(branch) => {
                    return branch.child_table.table_get(other.leaf_key()[O::key_index(self_depth)])
                        .and_then(|self_child| other.intersect(self_child, self_depth));
                }
            }
        }

        if other_depth < self_depth {
            // This means that there can be at most one child in other
            // that might intersect with self.
            match other.body() {
                BodyRef::Leaf(_) => unreachable!(),
                // If the depth of other is less than the depth of self, then it can't be a leaf.
                BodyRef::Branch(other_branch) => {
                    return other_branch.child_table.table_get(self.leaf_key()[O::key_index(other_depth)])
                        .and_then(|other_child| self.intersect(other_child, other_depth));
                }
            }
        }

        match (self.body(), other.body()) {
            (BodyRef::Branch(self_branch), BodyRef::Branch(other_branch)) => {
                let mut intersected_children = self_branch.child_table.iter()
                .filter_map(Option::as_ref)
                .cloned()
                .filter_map(|self_child| {
                    let other_child = other_branch.child_table.table_get(self_child.key())?;
                    self_child.intersect(other_child, self_depth)
                });
                let first_child = intersected_children.next()?;
                let Some(second_child) = intersected_children.next() else {
                    return Some(first_child);
                };
                let mut new_self = Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>::new2(
                    // This will be set later, because we don't know the key yet.
                    // The intersection might remove multiple levels of branches,
                    // so we can't just take the key from self or other.
                    0,
                    self_depth,
                    first_child.with_start(self_depth),
                    second_child.with_start(self_depth),
                );
                for child in intersected_children {
                    new_self.upsert(
               child.with_start(self_depth),
                 |_, _| unreachable!());
                }
                Some(new_self)
            }
            _ => unreachable!(),
            // If we reached this point then the depths are equal. The only way to have a leaf
            // is if the other is a leaf as well, which is already handled by the hash check if they are equal,
            // and by the key check if they are not equal.
            // If one of them is a leaf and the other is a branch, then they would also have different depths,
            // which is already handled by the above code.
        }
    }

    pub(crate) fn difference(&self, _other: &Self, _at_depth: usize) -> Option<Self> {
        todo!()
    }
}

unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> ByteEntry
    for Head<KEY_LEN, O, S>
{
    fn key(&self) -> u8 {
        self.key()
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> fmt::Debug
    for Head<KEY_LEN, O, S>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag().fmt(f)
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Clone
    for Head<KEY_LEN, O, S>
{
    fn clone(&self) -> Self {
        unsafe {
            match self.tag() {
                HeadTag::Leaf => {
                    Self::new(self.key(), Leaf::<KEY_LEN>::rc_inc(self.ptr()))
                }
                HeadTag::Branch2 => Self::new(
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch4 => Self::new(
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch8 => Self::new(
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch16 => Self::new(
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch32 => Self::new(
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch64 => Self::new(
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch128 => Self::new(
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch256 => Self::new(
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 256]>::rc_inc(self.ptr()),
                ),
            }
        }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Drop
    for Head<KEY_LEN, O, S>
{
    fn drop(&mut self) {
        unsafe {
            match self.tag() {
                HeadTag::Leaf => Leaf::<KEY_LEN>::rc_dec(self.ptr()),
                HeadTag::Branch2 => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>::rc_dec(self.ptr())
                }
                HeadTag::Branch4 => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>::rc_dec(self.ptr())
                }
                HeadTag::Branch8 => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>::rc_dec(self.ptr())
                }
                HeadTag::Branch16 => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>::rc_dec(self.ptr())
                }
                HeadTag::Branch32 => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>::rc_dec(self.ptr())
                }
                HeadTag::Branch64 => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>::rc_dec(self.ptr())
                }
                HeadTag::Branch128 => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>::rc_dec(self.ptr())
                }
                HeadTag::Branch256 => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 256]>::rc_dec(self.ptr())
                }
            }
        }
    }
}

/// A PATCH is a persistent data structure that stores a set of keys.
/// Each key can be reordered and segmented, based on the provided key ordering and segmentation.
///
/// The patch supports efficient set operations, like union, intersection, and difference,
/// because it efficiently maintains a hash for all keys that are part of a sub-tree.
///
/// The tree itself is a path- and node-compressed a 256-ary trie.
/// Each nodes stores its children in a byte oriented cuckoo hash table,
/// allowing for O(1) access to children, while keeping the memory overhead low.
/// Table sizes are powers of two, starting at 2.
///
/// Having a single node type for all branching factors simplifies the implementation,
/// compared to other adaptive trie implementations, like ARTs or Judy Arrays
///
/// The PATCH allows for cheap copy-on-write operations, with `clone` being O(1).
#[derive(Debug, Clone)]
pub struct PATCH<
    const KEY_LEN: usize,
    O: KeyOrdering<KEY_LEN> = IdentityOrder,
    S: KeySegmentation<KEY_LEN> = SingleSegmentation,
> {
    root: Option<Head<KEY_LEN, O, S>>,
}

impl<const KEY_LEN: usize, O, S> PATCH<KEY_LEN, O, S>
where
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
{
    /// Creates a new empty PATCH.
    pub fn new() -> Self {
        init_sip_key();
        PATCH { root: None }
    }

    /// Inserts a shared key into the PATCH.
    ///
    /// Takes an [Entry] object that can be created from a key,
    /// and inserted into multiple PATCH instances.
    ///
    /// If the key is already present, this is a no-op.
    pub fn insert(&mut self, entry: &Entry<KEY_LEN>) {
        if let Some(root) = &mut self.root {
            root.insert_leaf(entry.leaf(), 0);
        } else {
            self.root.replace(entry.leaf());
        }
    }

    /// Removes a key from the PATCH.
    ///
    /// If the key is not present, this is a no-op.
    pub fn remove(&mut self, key: &[u8; KEY_LEN]) {
        Head::remove_leaf(&mut self.root, key, 0);
    }

    /// Returns the number of keys in the PATCH.
    pub fn len(&self) -> u64 {
        if let Some(root) = &self.root {
            root.count()
        } else {
            0
        }
    }

    /// Allows iteratig over all infixes of a given length with a given prefix.
    /// Each infix is passed to the provided closure.
    ///
    /// The entire operation is performed over the tree view ordering of the keys.
    ///
    /// The length of the prefix and the infix is provided as type parameters,
    /// but will usually inferred from the arguments.
    ///
    /// The sum of `PREFIX_LEN` and `INFIX_LEN` must be less than or equal to `KEY_LEN`
    /// or a panic will occur.
    ///
    /// Because all infixes are iterated in one go, less bookkeeping is required,
    /// than when using an Iterator, allowing for better performance.
    pub fn infixes<const PREFIX_LEN: usize, const INFIX_LEN: usize, F>(
        &self,
        prefix: &[u8; PREFIX_LEN],
        mut for_each: F,
    ) where
        F: FnMut(&[u8; INFIX_LEN]),
    {
        assert!(
            PREFIX_LEN + INFIX_LEN <= KEY_LEN,
            "{} + {} > {}",
            PREFIX_LEN,
            INFIX_LEN,
            KEY_LEN
        );
        assert!(
            S::segment(O::key_index(PREFIX_LEN))
                == S::segment(O::key_index(PREFIX_LEN + INFIX_LEN - 1)),
            "PREFIX_LEN = {}, INFIX_LEN = {}, {} != {}",
            PREFIX_LEN,
            INFIX_LEN,
            S::segment(O::key_index(PREFIX_LEN)),
            S::segment(O::key_index(PREFIX_LEN + INFIX_LEN - 1))
        );
        if let Some(root) = &self.root {
            root.infixes(prefix, 0, &mut for_each);
        }
    }

    /// Returns true if the PATCH has a key with the given prefix.
    pub fn has_prefix<const PREFIX_LEN: usize>(&self, prefix: &[u8; PREFIX_LEN]) -> bool {
        if let Some(root) = &self.root {
            root.has_prefix(0, prefix)
        } else {
            PREFIX_LEN == 0
        }
    }

    /// Returns the number of unique segments in keys with the given prefix.
    pub fn segmented_len<const PREFIX_LEN: usize>(&self, prefix: &[u8; PREFIX_LEN]) -> u64 {
        if let Some(root) = &self.root {
            root.segmented_len(0, prefix)
        } else {
            0
        }
    }

    /// Iterates over all keys in the PATCH.
    /// The keys are returned in key order.
    pub fn iter<'a>(&'a self) -> PATCHIterator<'a, KEY_LEN, O, S> {
        PATCHIterator::new(self)
    }

    /// Iterate over all prefixes of the given length in the PATCH.
    /// The prefixes are naturally returned in tree order.
    /// A count of the number of elements for the given prefix is also returned.
    pub fn iter_prefix_count<'a, const PREFIX_LEN: usize>(
        &'a self,
    ) -> PATCHPrefixIterator<'a, KEY_LEN, PREFIX_LEN, O, S> {
        PATCHPrefixIterator::new(self)
    }

    /// Unions this PATCH with another PATCH.
    ///
    /// The other PATCH is consumed, and this PATCH is updated in place.
    pub fn union(&mut self, other: Self) {
        if let Some(other) = other.root {
            if let Some(root) = &mut self.root {
                root.union(other, 0);
            } else {
                self.root.replace(other);
            }
        }
    }

    /// Intersects this PATCH with another PATCH.
    ///
    /// Returns a new PATCH that contains only the keys that are present in both PATCHes.
    pub fn intersect(&self, other: &Self) -> Self {
        if let Some(root) = &self.root {
            if let Some(other_root) = &other.root {
                return Self {
                    root: root.intersect(other_root, 0).map(|root| root.with_start(0)),
                };
            }
        }
        Self::new()
    }

    /// Returns the difference between this PATCH and another PATCH.
    ///
    /// Returns a new PATCH that contains only the keys that are present in this PATCH,
    /// but not in the other PATCH.
    pub fn difference(&self, other: &Self) -> Self {
        if let Some(root) = &self.root {
            if let Some(other_root) = &other.root {
                return Self {
                    root: root.difference(other_root, 0),
                };
            } else {
                return self.clone();
            }
        } else {
            return other.clone();
        }
    }
}

impl<const KEY_LEN: usize, O, S> PartialEq for PATCH<KEY_LEN, O, S>
where
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
{
    fn eq(&self, other: &Self) -> bool {
        self.root.as_ref().map(|root| root.hash()) == other.root.as_ref().map(|root| root.hash())
    }
}

impl<const KEY_LEN: usize, O, S> Eq for PATCH<KEY_LEN, O, S>
where
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
{
}

impl<'a, const KEY_LEN: usize, O, S> IntoIterator for &'a PATCH<KEY_LEN, O, S>
where
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
{
    type Item = &'a [u8; KEY_LEN];
    type IntoIter = PATCHIterator<'a, KEY_LEN, O, S>;

    fn into_iter(self) -> Self::IntoIter {
        PATCHIterator::new(self)
    }
}

/// An iterator over all keys in a PATCH.
/// The keys are returned in key order.s
pub struct PATCHIterator<
    'a,
    const KEY_LEN: usize,
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
> {
    patch: PhantomData<&'a PATCH<KEY_LEN, O, S>>,
    stack: Vec<std::slice::Iter<'a, Option<Head<KEY_LEN, O, S>>>>,
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    PATCHIterator<'a, KEY_LEN, O, S>
{
    fn new(patch: &'a PATCH<KEY_LEN, O, S>) -> Self {
        let mut r = PATCHIterator {
            patch: PhantomData,
            stack: Vec::new(),
        };
        r.stack.push(std::slice::from_ref(&patch.root).iter());
        r
    }
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Iterator
    for PATCHIterator<'a, KEY_LEN, O, S>
{
    type Item = &'a [u8; KEY_LEN];

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.stack.pop()?;
        loop {
            if let Some(child) = iter.next() {
                if let Some(child) = child {
                    match child.body() {
                        BodyRef::Leaf(leaf) => {
                            self.stack.push(iter);
                            return Some(&leaf.key);
                        },
                        BodyRef::Branch(branch) => {
                            self.stack.push(iter);
                            iter = branch.child_table.iter();
                        },
                    }
                }
            } else {
                iter = self.stack.pop()?;
            }
        }
    }
}

/// An iterator over all keys in a PATCH that have a given prefix.
/// The keys are returned in tree order.
pub struct PATCHPrefixIterator<
    'a,
    const KEY_LEN: usize,
    const PREFIX_LEN: usize,
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
> {
    patch: PhantomData<&'a PATCH<KEY_LEN, O, S>>,
    stack: Vec<Vec<&'a Head<KEY_LEN, O, S>>>,
}

impl<
        'a,
        const KEY_LEN: usize,
        const PREFIX_LEN: usize,
        O: KeyOrdering<KEY_LEN>,
        S: KeySegmentation<KEY_LEN>,
    > PATCHPrefixIterator<'a, KEY_LEN, PREFIX_LEN, O, S>
{
    fn new(patch: &'a PATCH<KEY_LEN, O, S>) -> Self {
        assert!(PREFIX_LEN <= KEY_LEN);
        let mut r = PATCHPrefixIterator {
            patch: PhantomData,
            stack: Vec::with_capacity(PREFIX_LEN),
        };
        if let Some(root) = &patch.root {
            let mut level = Vec::with_capacity(256);
            level.push(root);
            r.stack.push(level);
        }
        r
    }
}

impl<
        'a,
        const KEY_LEN: usize,
        const PREFIX_LEN: usize,
        O: KeyOrdering<KEY_LEN>,
        S: KeySegmentation<KEY_LEN>,
    > Iterator for PATCHPrefixIterator<'a, KEY_LEN, PREFIX_LEN, O, S>
{
    type Item = ([u8; PREFIX_LEN], u64);

    fn next(&mut self) -> Option<Self::Item> {
        let mut level = self.stack.pop()?;
        loop {
            if let Some(child) = level.pop() {
                if child.end_depth() >= PREFIX_LEN {
                    let key = O::tree_ordered(child.leaf_key());
                    let suffix_count = child.count();
                    self.stack.push(level);
                    return Some((key[0..PREFIX_LEN].try_into().unwrap(), suffix_count));
                } else {
                    self.stack.push(level);
                    match child.body() {
                        BodyRef::Leaf(_) => panic!("iter_children on leaf"),
                        BodyRef::Branch(branch) => {
                            level = branch.child_table.iter().filter_map(|c| c.as_ref()).collect();
                            level.sort_by_key(|&k| Reverse(k.key())); // We need to reverse here because we pop from the vec.
                        },
                    }
                }
            } else {
                level = self.stack.pop()?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::convert::TryInto;
    use std::iter::FromIterator;
    use std::mem;

    #[test]
    fn head_tag() {
        let head = Head::<64, IdentityOrder, SingleSegmentation>::new::<Leaf<64>>(
                0,
                NonNull::dangling(),
            );
        assert_eq!(head.tag(), HeadTag::Leaf);
        mem::forget(head);
    }

    #[test]
    fn head_key() {
        for k in 0..=255 {
            let head = Head::<64, IdentityOrder, SingleSegmentation>::new::<Leaf<64>>(
                    k,
                    NonNull::dangling(),
                );
            assert_eq!(head.key(), k);
            mem::forget(head);
        }
    }

    #[test]
    fn head_size() {
        assert_eq!(
            mem::size_of::<Head<64, IdentityOrder, SingleSegmentation>>(),
            8
        );
    }

    #[test]
    fn empty_tree() {
        let _tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
    }

    #[test]
    fn tree_put_one() {
        const KEY_SIZE: usize = 64;
        let mut tree = PATCH::<KEY_SIZE, IdentityOrder, SingleSegmentation>::new();
        let entry = Entry::new(&[0; KEY_SIZE]);
        tree.insert(&entry);
    }

    #[test]
    fn tree_put_same() {
        const KEY_SIZE: usize = 64;
        let mut tree = PATCH::<KEY_SIZE, IdentityOrder, SingleSegmentation>::new();
        let entry = Entry::new(&[0; KEY_SIZE]);
        tree.insert(&entry);
        tree.insert(&entry);
    }

    #[test]
    fn branch_size() {
        assert_eq!(
            mem::size_of::<
                Branch<
                    64,
                    IdentityOrder,
                    SingleSegmentation,
                    [Option<Head<64, IdentityOrder, SingleSegmentation>>; 2],
                >,
            >(),
            64
        );
        assert_eq!(
            mem::size_of::<
                Branch<
                    64,
                    IdentityOrder,
                    SingleSegmentation,
                    [Option<Head<64, IdentityOrder, SingleSegmentation>>; 4],
                >,
            >(),
            48 + 16 * 2
        );
        assert_eq!(
            mem::size_of::<
                Branch<
                    64,
                    IdentityOrder,
                    SingleSegmentation,
                    [Option<Head<64, IdentityOrder, SingleSegmentation>>; 8],
                >,
            >(),
            48 + 16 * 4
        );
        assert_eq!(
            mem::size_of::<
                Branch<
                    64,
                    IdentityOrder,
                    SingleSegmentation,
                    [Option<Head<64, IdentityOrder, SingleSegmentation>>; 16],
                >,
            >(),
            48 + 16 * 8
        );
        assert_eq!(
            mem::size_of::<
                Branch<
                    64,
                    IdentityOrder,
                    SingleSegmentation,
                    [Option<Head<32, IdentityOrder, SingleSegmentation>>; 32],
                >,
            >(),
            48 + 16 * 16
        );
        assert_eq!(
            mem::size_of::<
                Branch<
                    64,
                    IdentityOrder,
                    SingleSegmentation,
                    [Option<Head<64, IdentityOrder, SingleSegmentation>>; 64],
                >,
            >(),
            48 + 16 * 32
        );
        assert_eq!(
            mem::size_of::<
                Branch<
                    64,
                    IdentityOrder,
                    SingleSegmentation,
                    [Option<Head<64, IdentityOrder, SingleSegmentation>>; 128],
                >,
            >(),
            48 + 16 * 64
        );
        assert_eq!(
            mem::size_of::<
                Branch<
                    64,
                    IdentityOrder,
                    SingleSegmentation,
                    [Option<Head<64, IdentityOrder, SingleSegmentation>>; 256],
                >,
            >(),
            48 + 16 * 128
        );
    }

    /// Checks what happens if we join two PATCHes that
    /// only contain a single element each, that differs in the last byte.
    #[test]
    fn tree_union_single() {
        const KEY_SIZE: usize = 8;
        let mut left = PATCH::<KEY_SIZE, IdentityOrder, SingleSegmentation>::new();
        let mut right = PATCH::<KEY_SIZE, IdentityOrder, SingleSegmentation>::new();
        let left_entry = Entry::new(&[0, 0, 0, 0, 0, 0, 0, 0]);
        let right_entry = Entry::new(&[0, 0, 0, 0, 0, 0, 0, 1]);
        left.insert(&left_entry);
        right.insert(&right_entry);
        left.union(right);
        assert_eq!(left.len(), 2);
    }

    proptest! {
    #[test]
    fn tree_insert(keys in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
        let mut tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        for key in keys {
            let key: [u8; 64] = key.try_into().unwrap();
            let entry = Entry::new(&key);
            tree.insert(&entry);
        }
    }

    #[test]
    fn tree_len(keys in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
        let mut tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        let mut set = HashSet::new();
        for key in keys {
            let key: [u8; 64] = key.try_into().unwrap();
            let entry = Entry::new(&key);
            tree.insert(&entry);
            set.insert(key);
        }

        prop_assert_eq!(set.len() as u64, tree.len())
    }

    #[test]
    fn tree_infixes(keys in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
        let mut tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        let mut set = HashSet::new();
        for key in keys {
            let key: [u8; 64] = key.try_into().unwrap();
            let entry = Entry::new(&key);
            tree.insert(&entry);
            set.insert(key);
        }
        let mut set_vec = Vec::from_iter(set.into_iter());
        let mut tree_vec = vec![];
        tree.infixes(&[0; 0], &mut |&x: &[u8; 64]| tree_vec.push(x));

        set_vec.sort();
        tree_vec.sort();

        prop_assert_eq!(set_vec, tree_vec);
    }

    #[test]
    fn tree_iter(keys in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
        let mut tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        let mut set = HashSet::new();
        for key in keys {
            let key: [u8; 64] = key.try_into().unwrap();
            let entry = Entry::new(&key);
            tree.insert(&entry);
            set.insert(key);
        }
        let mut set_vec = Vec::from_iter(set.into_iter());
        let mut tree_vec = vec![];
        for key in &tree {
            tree_vec.push(*key);
        }

        set_vec.sort();
        tree_vec.sort();

        prop_assert_eq!(set_vec, tree_vec);
    }

    #[test]
    fn tree_union(left in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 2000),
                    right in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 2000)) {
        let mut set = HashSet::new();

        let mut left_tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        for entry in left {
            let mut key = [0; 64];
            key.iter_mut().set_from(entry.iter().cloned());
            let entry = Entry::new(&key);
            left_tree.insert(&entry);
            set.insert(key);
        }

        let mut right_tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        for entry in right {
            let mut key = [0; 64];
            key.iter_mut().set_from(entry.iter().cloned());
            let entry = Entry::new(&key);
            right_tree.insert(&entry);
            set.insert(key);
        }

        left_tree.union(right_tree);

        let mut set_vec = Vec::from_iter(set.into_iter());
        let mut tree_vec = vec![];
        left_tree.infixes(&[0; 0], &mut |&x: &[u8;64]| tree_vec.push(x));

        set_vec.sort();
        tree_vec.sort();

        prop_assert_eq!(set_vec, tree_vec);
        }
        #[test]
    fn tree_union_empty(left in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 2)) {
        let mut set = HashSet::new();

        let mut left_tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        for entry in left {
            let mut key = [0; 64];
            key.iter_mut().set_from(entry.iter().cloned());
            let entry = Entry::new(&key);
            left_tree.insert(&entry);
            set.insert(key);
        }

        let right_tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();

        left_tree.union(right_tree);

        let mut set_vec = Vec::from_iter(set.into_iter());
        let mut tree_vec = vec![];
        left_tree.infixes(&[0; 0], &mut |&x: &[u8;64]| tree_vec.push(x));

        set_vec.sort();
        tree_vec.sort();

        prop_assert_eq!(set_vec, tree_vec);
        }
    }
}
