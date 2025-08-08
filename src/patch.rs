//! Persistent Adaptive Trie with Cuckoo-compression and
//! Hash-maintenance (PATCH).
//!
//! See the [PATCH](../book/src/deep-dive/patch.md) chapter of the Tribles Book
//! for the full design description and hashing scheme.
//!
#![allow(unstable_name_collisions)]

mod branch;
pub mod bytetable;
mod entry;
mod leaf;

use arrayvec::ArrayVec;

use branch::*;
pub use entry::Entry;
use leaf::*;

pub use bytetable::*;
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

/// Builds a per-byte segment map from the segment lengths.
///
/// The returned table maps each key byte to its segment index.
pub const fn build_segmentation<const N: usize, const M: usize>(lens: [usize; M]) -> [usize; N] {
    let mut res = [0; N];
    let mut seg = 0;
    let mut off = 0;
    while seg < M {
        let len = lens[seg];
        let mut i = 0;
        while i < len {
            res[off + i] = seg;
            i += 1;
        }
        off += len;
        seg += 1;
    }
    res
}

/// Builds an identity permutation table of length `N`.
pub const fn identity_map<const N: usize>() -> [usize; N] {
    let mut res = [0; N];
    let mut i = 0;
    while i < N {
        res[i] = i;
        i += 1;
    }
    res
}

/// Builds a table translating indices from key order to tree order.
///
/// `lens` describes the segment lengths in key order and `perm` is the
/// permutation of those segments in tree order.
pub const fn build_key_to_tree<const N: usize, const M: usize>(
    lens: [usize; M],
    perm: [usize; M],
) -> [usize; N] {
    let mut key_starts = [0; M];
    let mut off = 0;
    let mut i = 0;
    while i < M {
        key_starts[i] = off;
        off += lens[i];
        i += 1;
    }

    let mut tree_starts = [0; M];
    off = 0;
    i = 0;
    while i < M {
        let seg = perm[i];
        tree_starts[seg] = off;
        off += lens[seg];
        i += 1;
    }

    let mut res = [0; N];
    let mut seg = 0;
    while seg < M {
        let len = lens[seg];
        let ks = key_starts[seg];
        let ts = tree_starts[seg];
        let mut j = 0;
        while j < len {
            res[ks + j] = ts + j;
            j += 1;
        }
        seg += 1;
    }
    res
}

/// Inverts a permutation table.
pub const fn invert<const N: usize>(arr: [usize; N]) -> [usize; N] {
    let mut res = [0; N];
    let mut i = 0;
    while i < N {
        res[arr[i]] = i;
        i += 1;
    }
    res
}

#[doc(hidden)]
#[macro_export]
macro_rules! key_segmentation {
    (@count $($e:expr),* $(,)?) => {
        <[()]>::len(&[$($crate::key_segmentation!(@sub $e)),*])
    };
    (@sub $e:expr) => { () };
    ($name:ident, $len:expr, [$($seg_len:expr),+ $(,)?]) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $name;
        impl $name {
            pub const SEG_LENS: [usize; $crate::key_segmentation!(@count $($seg_len),*)] = [$($seg_len),*];
        }
        impl $crate::patch::KeySegmentation<$len> for $name {
            const SEGMENTS: [usize; $len] = $crate::patch::build_segmentation::<$len, {$crate::key_segmentation!(@count $($seg_len),*)}>(Self::SEG_LENS);
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! key_ordering {
    (@count $($e:expr),* $(,)?) => {
        <[()]>::len(&[$($crate::key_ordering!(@sub $e)),*])
    };
    (@sub $e:expr) => { () };
    ($name:ident, $seg:ty, $len:expr, [$($perm:expr),+ $(,)?]) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $name;
        impl $crate::patch::KeyOrdering<$len> for $name {
            type Segmentation = $seg;
            const SEGMENT_PERM: &'static [usize] = &[$($perm),*];
            const KEY_TO_TREE: [usize; $len] = $crate::patch::build_key_to_tree::<$len, {$crate::key_ordering!(@count $($perm),*)}>(<$seg>::SEG_LENS, [$($perm),*]);
            const TREE_TO_KEY: [usize; $len] = $crate::patch::invert(Self::KEY_TO_TREE);
        }
    };
}

/// A trait is used to provide a re-ordered view of the keys stored in the PATCH.
/// This allows for different PATCH instances share the same leaf nodes,
/// independent of the key ordering used in the tree.
pub trait KeyOrdering<const KEY_LEN: usize>: Copy + Clone + Debug {
    /// The segmentation this ordering operates over.
    type Segmentation: KeySegmentation<KEY_LEN>;
    /// Order of segments from key layout to tree layout.
    const SEGMENT_PERM: &'static [usize];
    /// Maps each key index to its position in the tree view.
    const KEY_TO_TREE: [usize; KEY_LEN];
    /// Maps each tree index to its position in the key view.
    const TREE_TO_KEY: [usize; KEY_LEN];

    /// Reorders the key from the shared key ordering to the tree ordering.
    fn tree_ordered(key: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
        let mut new_key = [0; KEY_LEN];
        let mut i = 0;
        while i < KEY_LEN {
            new_key[Self::KEY_TO_TREE[i]] = key[i];
            i += 1;
        }
        new_key
    }

    /// Reorders the key from the tree ordering to the shared key ordering.
    fn key_ordered(tree_key: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
        let mut new_key = [0; KEY_LEN];
        let mut i = 0;
        while i < KEY_LEN {
            new_key[Self::TREE_TO_KEY[i]] = tree_key[i];
            i += 1;
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
    /// Segment index for each position in the key.
    const SEGMENTS: [usize; KEY_LEN];
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
    type Segmentation = SingleSegmentation;
    const SEGMENT_PERM: &'static [usize] = &[0];
    const KEY_TO_TREE: [usize; KEY_LEN] = identity_map::<KEY_LEN>();
    const TREE_TO_KEY: [usize; KEY_LEN] = identity_map::<KEY_LEN>();
}

impl<const KEY_LEN: usize> KeySegmentation<KEY_LEN> for SingleSegmentation {
    const SEGMENTS: [usize; KEY_LEN] = [0; KEY_LEN];
}
#[allow(dead_code)]
#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub(crate) enum HeadTag {
    // Bit 0-3: Branching factor
    Branch2 = 1,
    Branch4 = 2,
    Branch8 = 3,
    Branch16 = 4,
    Branch32 = 5,
    Branch64 = 6,
    Branch128 = 7,
    Branch256 = 8,
    // Bit 4 indicates that the node is a leaf.
    Leaf = 16,
}

pub(crate) enum BodyPtr<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> {
    Leaf(NonNull<Leaf<KEY_LEN>>),
    Branch(NonNull<Branch<KEY_LEN, O, [Option<Head<KEY_LEN, O>>]>>),
}

pub(crate) trait Body {
    fn tag(body: NonNull<Self>) -> HeadTag;
}

#[repr(C)]
pub(crate) struct Head<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> {
    tptr: std::ptr::NonNull<u8>,
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<O::Segmentation>,
}

unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> Send for Head<KEY_LEN, O> {}
unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> Sync for Head<KEY_LEN, O> {}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> Head<KEY_LEN, O> {
    pub(crate) fn new<T: Body + ?Sized>(key: u8, body: NonNull<T>) -> Self {
        unsafe {
            let tptr =
                std::ptr::NonNull::new_unchecked((body.as_ptr() as *mut u8).map_addr(|addr| {
                    ((addr as u64 & 0x00_00_ff_ff_ff_ff_ff_ffu64)
                        | ((key as u64) << 48)
                        | ((<T as Body>::tag(body) as u64) << 56)) as usize
                }));
            Self {
                tptr,
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
    pub(crate) fn with_key(mut self, key: u8) -> Self {
        self.tptr = std::ptr::NonNull::new(self.tptr.as_ptr().map_addr(|addr| {
            ((addr as u64 & 0xff_00_ff_ff_ff_ff_ff_ffu64) | ((key as u64) << 48)) as usize
        }))
        .unwrap();
        self
    }

    #[inline]
    pub(crate) fn set_body<T: Body + ?Sized>(&mut self, body: NonNull<T>) {
        unsafe {
            self.tptr = NonNull::new_unchecked((body.as_ptr() as *mut u8).map_addr(|addr| {
                ((addr as u64 & 0x00_00_ff_ff_ff_ff_ff_ffu64)
                    | (self.tptr.as_ptr() as u64 & 0x00_ff_00_00_00_00_00_00u64)
                    | ((<T as Body>::tag(body) as u64) << 56)) as usize
            }))
        }
    }

    pub(crate) fn with_start(self, new_start_depth: usize) -> Head<KEY_LEN, O> {
        let leaf_key = self.childleaf_key();
        let i = O::TREE_TO_KEY[new_start_depth];
        let key = leaf_key[i];
        self.with_key(key)
    }

    pub(crate) fn body(&self) -> BodyPtr<KEY_LEN, O> {
        unsafe {
            let ptr = NonNull::new_unchecked(
                self.tptr
                    .as_ptr()
                    .map_addr(|addr| ((((addr as u64) << 16) as i64) >> 16) as usize),
            );
            match self.tag() {
                HeadTag::Leaf => BodyPtr::Leaf(ptr.cast()),
                branch_tag => {
                    let count = 1 << (branch_tag as usize);
                    BodyPtr::Branch(NonNull::new_unchecked(std::ptr::slice_from_raw_parts(
                        ptr.as_ptr(),
                        count,
                    )
                        as *mut Branch<KEY_LEN, O, [Option<Head<KEY_LEN, O>>]>))
                }
            }
        }
    }

    pub(crate) fn body_mut(&mut self) -> BodyPtr<KEY_LEN, O> {
        unsafe {
            match self.body() {
                BodyPtr::Leaf(leaf) => BodyPtr::Leaf(leaf),
                BodyPtr::Branch(branch) => {
                    if let Some(copy) = Branch::rc_cow(branch) {
                        self.set_body(copy);
                        BodyPtr::Branch(copy)
                    } else {
                        BodyPtr::Branch(branch)
                    }
                }
            }
        }
    }

    pub(crate) fn count(&self) -> u64 {
        match self.body() {
            BodyPtr::Leaf(_) => 1,
            BodyPtr::Branch(branch) => unsafe { branch.as_ref().leaf_count },
        }
    }

    pub(crate) fn count_segment(&self, at_depth: usize) -> u64 {
        match self.body() {
            BodyPtr::Leaf(_) => 1,
            BodyPtr::Branch(branch) => Branch::count_segment(branch, at_depth),
        }
    }

    pub(crate) fn hash(&self) -> u128 {
        match self.body() {
            BodyPtr::Leaf(leaf) => unsafe { leaf.as_ref().hash },
            BodyPtr::Branch(branch) => unsafe { branch.as_ref().hash },
        }
    }

    pub(crate) fn end_depth(&self) -> usize {
        match self.body() {
            BodyPtr::Leaf(_) => KEY_LEN as usize,
            BodyPtr::Branch(branch) => unsafe { branch.as_ref().end_depth as usize },
        }
    }

    pub(crate) fn childleaf(&self) -> *const Leaf<KEY_LEN> {
        match self.body() {
            BodyPtr::Leaf(leaf) => leaf.as_ptr(),
            BodyPtr::Branch(branch) => unsafe { branch.as_ref().childleaf },
        }
    }

    pub(crate) fn childleaf_key<'a>(&'a self) -> &'a [u8; KEY_LEN] {
        unsafe { &(*self.childleaf()).key }
    }

    pub(crate) fn remove_leaf(
        slot: &mut Option<Self>,
        leaf_key: &[u8; KEY_LEN],
        start_depth: usize,
    ) {
        if let Some(this) = slot {
            let head_key = this.childleaf_key();

            let end_depth = std::cmp::min(this.end_depth(), KEY_LEN);
            for depth in start_depth..end_depth {
                let i = O::TREE_TO_KEY[depth];
                if head_key[i] != leaf_key[i] {
                    return;
                }
            }
            match this.body_mut() {
                BodyPtr::Leaf(_) => {
                    slot.take();
                }
                BodyPtr::Branch(mut branch) => {
                    let branch = unsafe { branch.as_mut() };

                    let key = leaf_key[end_depth];
                    let removed_leafchild = leaf_key == unsafe { &(*branch.childleaf).key };
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
                                            let child = branch
                                                .child_table
                                                .iter()
                                                .find_map(|s| s.as_ref())
                                                .expect("child should exist");
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

    pub(crate) fn insert_leaf(slot: &mut Option<Self>, leaf: Self, start_depth: usize) {
        let this = slot.as_mut().expect("slot should not be empty");
        let this_key = this.childleaf_key();
        let leaf_key = leaf.childleaf_key();

        let end_depth = std::cmp::min(this.end_depth(), KEY_LEN);
        for depth in start_depth..end_depth {
            let i = O::TREE_TO_KEY[depth];
            let this_byte_key = this_key[i];
            let leaf_byte_key = leaf_key[i];
            if this_byte_key != leaf_byte_key {
                let old_key = this.key();
                let new_body = Branch::new(
                    depth,
                    slot.take().unwrap().with_key(this_byte_key),
                    leaf.with_key(leaf_byte_key),
                );

                if slot.replace(Head::new(old_key, new_body)).is_some() {
                    unreachable!();
                }

                return;
            }
        }

        if end_depth != KEY_LEN {
            let BodyPtr::Branch(body) = this.body_mut() else {
                unreachable!();
            };
            let new_body = Branch::upsert_child(body, leaf, |slot, inserted| {
                Head::insert_leaf(slot, inserted, end_depth)
            });
            this.set_body(new_body);
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
            BodyPtr::Leaf(leaf) => {
                Leaf::infixes::<PREFIX_LEN, INFIX_LEN, O, F>(leaf, prefix, at_depth, f)
            }
            BodyPtr::Branch(branch) => Branch::infixes(branch, prefix, at_depth, f),
        }
    }

    pub(crate) fn has_prefix<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        const {
            assert!(PREFIX_LEN <= KEY_LEN);
        }
        match self.body() {
            BodyPtr::Leaf(leaf) => Leaf::has_prefix::<O, PREFIX_LEN>(leaf, at_depth, prefix),
            BodyPtr::Branch(branch) => branch::Branch::has_prefix(branch, at_depth, prefix),
        }
    }

    pub(crate) fn segmented_len<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        match self.body() {
            BodyPtr::Leaf(leaf) => Leaf::segmented_len::<O, PREFIX_LEN>(leaf, at_depth, prefix),
            BodyPtr::Branch(branch) => Branch::segmented_len(branch, at_depth, prefix),
        }
    }

    pub(crate) fn union(slot: &mut Option<Self>, mut other: Self, at_depth: usize) {
        let this = slot.as_mut().expect("slot should not be empty");
        let this_hash = this.hash();
        let other_hash = other.hash();
        if this_hash == other_hash {
            return;
        }
        let this_depth = this.end_depth();
        let other_depth = other.end_depth();

        let this_key = this.childleaf_key();
        let other_key = other.childleaf_key();
        for depth in at_depth..std::cmp::min(this_depth, other_depth) {
            let i = O::TREE_TO_KEY[depth];
            let this_byte_key = this_key[i];
            let other_byte_key = other_key[i];
            if this_byte_key != other_byte_key {
                let old_key = this.key();
                let new_body = Branch::new(
                    depth,
                    slot.take().unwrap().with_key(this_byte_key),
                    other.with_key(other_byte_key),
                );

                if slot.replace(Head::new(old_key, new_body)).is_some() {
                    unreachable!();
                }

                return;
            }
        }

        if this_depth < other_depth {
            match this.body_mut() {
                BodyPtr::Leaf(_) => unreachable!(),
                BodyPtr::Branch(body) => {
                    let new_body = Branch::upsert_child(body, other, |slot, inserted| {
                        Head::union(slot, inserted, this_depth)
                    });
                    this.set_body(new_body);
                }
            }

            return;
        }

        if other_depth < this_depth {
            let old_key = this.key();
            let this = slot.take().unwrap();
            match other.body_mut() {
                BodyPtr::Leaf(_) => unreachable!(),
                BodyPtr::Branch(body) => {
                    let new_body = Branch::upsert_child(body, this, |slot, inserted| {
                        Head::union(slot, inserted, other_depth)
                    });
                    other.set_body(new_body);
                }
            }

            if slot.replace(other.with_key(old_key)).is_some() {
                unreachable!();
            }
            return;
        }

        // we already checked for equality by comparing the hashes,
        // if they are not equal, the keys must be different which is
        // already handled by the above code.
        let BodyPtr::Branch(mut this_body) = this.body_mut() else {
            unreachable!();
        };
        let BodyPtr::Branch(mut other_branch) = other.body_mut() else {
            unreachable!();
        };
        unsafe {
            for other_child in other_branch
                .as_mut()
                .child_table
                .iter_mut()
                .filter_map(Option::take)
            {
                this_body = Branch::upsert_child(this_body, other_child, |slot, inserted| {
                    Head::union(slot, inserted, this_depth)
                });
            }
            this.set_body(this_body);
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

        let self_key = self.childleaf_key();
        let other_key = other.childleaf_key();
        for depth in at_depth..std::cmp::min(self_depth, other_depth) {
            let i = O::TREE_TO_KEY[depth];
            if self_key[i] != other_key[i] {
                return None;
            }
        }

        if self_depth < other_depth {
            // This means that there can be at most one child in self
            // that might intersect with other.
            let BodyPtr::Branch(branch) = self.body() else {
                unreachable!();
            };
            unsafe {
                return branch
                    .as_ref()
                    .child_table
                    .table_get(other.childleaf_key()[O::TREE_TO_KEY[self_depth]])
                    .and_then(|self_child| other.intersect(self_child, self_depth));
            }
        }

        if other_depth < self_depth {
            // This means that there can be at most one child in other
            // that might intersect with self.
            // If the depth of other is less than the depth of self, then it can't be a leaf.
            let BodyPtr::Branch(other_branch) = other.body() else {
                unreachable!();
            };
            unsafe {
                return other_branch
                    .as_ref()
                    .child_table
                    .table_get(self.childleaf_key()[O::TREE_TO_KEY[other_depth]])
                    .and_then(|other_child| self.intersect(other_child, other_depth));
            }
        }

        // If we reached this point then the depths are equal. The only way to have a leaf
        // is if the other is a leaf as well, which is already handled by the hash check if they are equal,
        // and by the key check if they are not equal.
        // If one of them is a leaf and the other is a branch, then they would also have different depths,
        // which is already handled by the above code.
        let BodyPtr::Branch(self_branch) = self.body() else {
            unreachable!();
        };
        let BodyPtr::Branch(other_branch) = other.body() else {
            unreachable!();
        };

        unsafe {
            let mut intersected_children = self_branch
                .as_ref()
                .child_table
                .iter()
                .filter_map(Option::as_ref)
                .filter_map(|self_child| {
                    let other_child = other_branch
                        .as_ref()
                        .child_table
                        .table_get(self_child.key())?;
                    self_child.intersect(other_child, self_depth)
                });
            let first_child = intersected_children.next()?;
            let Some(second_child) = intersected_children.next() else {
                return Some(first_child);
            };
            let mut new_branch = Branch::new(
                self_depth,
                first_child.with_start(self_depth),
                second_child.with_start(self_depth),
            );
            for child in intersected_children {
                new_branch = Branch::insert_child(new_branch, child.with_start(self_depth));
            }
            // The key will be set later, because we don't know it yet.
            // The intersection might remove multiple levels of branches,
            // so we can't just take the key from self or other.
            Some(Head::new(0, new_branch))
        }
    }

    /// Returns the difference between self and other.
    /// This is the set of elements that are in self but not in other.
    /// If the difference is empty, None is returned.
    pub(crate) fn difference(&self, other: &Self, at_depth: usize) -> Option<Self> {
        let self_hash = self.hash();
        let other_hash = other.hash();
        if self_hash == other_hash {
            return None;
        }
        let self_depth = self.end_depth();
        let other_depth = other.end_depth();

        let self_key = self.childleaf_key();
        let other_key = other.childleaf_key();
        for depth in at_depth..std::cmp::min(self_depth, other_depth) {
            let i = O::TREE_TO_KEY[depth];
            if self_key[i] != other_key[i] {
                return Some(self.clone());
            }
        }

        if self_depth < other_depth {
            // This means that there can be at most one child in self
            // that might intersect with other. It's the only child that may not be in the difference.
            // The other children are definitely in the difference, as they have no corresponding byte in other.
            // Thus the cheapest way to compute the difference is compute the difference of the only child
            // that might intersect with other, copy self with it's correctly filled byte table, then
            // remove the old child, and insert the new child.
            let mut new_branch = self.clone();
            let BodyPtr::Branch(branch) = new_branch.body_mut() else {
                unreachable!();
            };
            let other_byte_key = other.childleaf_key()[O::TREE_TO_KEY[self_depth]];
            Branch::update_child(branch, other_byte_key, |child| {
                child.difference(other, self_depth)
            });
            return Some(new_branch);
        }

        if other_depth < self_depth {
            // This means that we need to check if there is a child in other
            // that matches the path at the current depth of self.
            // There is no such child, then then self must be in the difference.
            // If there is such a child, then we have to compute the difference
            // between self and that child.
            // We know that other must be a branch.
            let BodyPtr::Branch(other_branch) = other.body() else {
                unreachable!();
            };
            let self_byte_key = self.childleaf_key()[O::TREE_TO_KEY[other_depth]];

            if let Some(other_child) =
                unsafe { other_branch.as_ref().child_table.table_get(self_byte_key) }
            {
                return self.difference(other_child, at_depth);
            } else {
                return Some(self.clone());
            }
        }

        // If we reached this point then the depths are equal. The only way to have a leaf
        // is if the other is a leaf as well, which is already handled by the hash check if they are equal,
        // and by the key check if they are not equal.
        // If one of them is a leaf and the other is a branch, then they would also have different depths,
        // which is already handled by the above code.
        let BodyPtr::Branch(self_branch) = self.body() else {
            unreachable!();
        };
        let BodyPtr::Branch(other_branch) = other.body() else {
            unreachable!();
        };

        unsafe {
            let mut differenced_children = self_branch
                .as_ref()
                .child_table
                .iter()
                .filter_map(Option::as_ref)
                .filter_map(|self_child| {
                    if let Some(other_child) = other_branch
                        .as_ref()
                        .child_table
                        .table_get(self_child.key())
                    {
                        self_child.difference(other_child, self_depth)
                    } else {
                        Some(self_child.clone())
                    }
                });
            let first_child = differenced_children.next()?;
            let Some(second_child) = differenced_children.next() else {
                return Some(first_child);
            };
            let mut new_branch = Branch::new(
                self_depth,
                first_child.with_start(self_depth),
                second_child.with_start(self_depth),
            );
            for child in differenced_children {
                new_branch = Branch::insert_child(new_branch, child.with_start(self_depth));
            }
            // The key will be set later, because we don't know it yet.
            // The difference might remove multiple levels of branches,
            // so we can't just take the key from self or other.
            Some(Head::new(0, new_branch))
        }
    }
}

unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> ByteEntry for Head<KEY_LEN, O> {
    fn key(&self) -> u8 {
        self.key()
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> fmt::Debug for Head<KEY_LEN, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag().fmt(f)
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> Clone for Head<KEY_LEN, O> {
    fn clone(&self) -> Self {
        unsafe {
            match self.body() {
                BodyPtr::Leaf(leaf) => Self::new(self.key(), Leaf::rc_inc(leaf)),
                BodyPtr::Branch(branch) => Self::new(self.key(), Branch::rc_inc(branch)),
            }
        }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> Drop for Head<KEY_LEN, O> {
    fn drop(&mut self) {
        unsafe {
            match self.body() {
                BodyPtr::Leaf(leaf) => Leaf::rc_dec(leaf),
                BodyPtr::Branch(branch) => Branch::rc_dec(branch),
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
pub struct PATCH<const KEY_LEN: usize, O = IdentityOrder>
where
    O: KeyOrdering<KEY_LEN>,
{
    root: Option<Head<KEY_LEN, O>>,
}

impl<const KEY_LEN: usize, O> PATCH<KEY_LEN, O>
where
    O: KeyOrdering<KEY_LEN>,
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
        if self.root.is_some() {
            Head::insert_leaf(&mut self.root, entry.leaf(), 0);
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
    /// or a compile-time assertion will fail.
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
        const {
            assert!(PREFIX_LEN + INFIX_LEN <= KEY_LEN);
        }
        assert!(
            <O as KeyOrdering<KEY_LEN>>::Segmentation::SEGMENTS[O::TREE_TO_KEY[PREFIX_LEN]]
                == <O as KeyOrdering<KEY_LEN>>::Segmentation::SEGMENTS
                    [O::TREE_TO_KEY[PREFIX_LEN + INFIX_LEN - 1]]
                && (PREFIX_LEN + INFIX_LEN == KEY_LEN
                    || <O as KeyOrdering<KEY_LEN>>::Segmentation::SEGMENTS
                        [O::TREE_TO_KEY[PREFIX_LEN + INFIX_LEN - 1]]
                        != <O as KeyOrdering<KEY_LEN>>::Segmentation::SEGMENTS
                            [O::TREE_TO_KEY[PREFIX_LEN + INFIX_LEN]]),
            "INFIX_LEN must cover a whole segment"
        );
        if let Some(root) = &self.root {
            root.infixes(prefix, 0, &mut for_each);
        }
    }

    /// Returns true if the PATCH has a key with the given prefix.
    ///
    /// `PREFIX_LEN` must be less than or equal to `KEY_LEN` or a compile-time
    /// assertion will fail.
    pub fn has_prefix<const PREFIX_LEN: usize>(&self, prefix: &[u8; PREFIX_LEN]) -> bool {
        const {
            assert!(PREFIX_LEN <= KEY_LEN);
        }
        if let Some(root) = &self.root {
            root.has_prefix(0, prefix)
        } else {
            PREFIX_LEN == 0
        }
    }

    /// Returns the number of unique segments in keys with the given prefix.
    pub fn segmented_len<const PREFIX_LEN: usize>(&self, prefix: &[u8; PREFIX_LEN]) -> u64 {
        const {
            assert!(PREFIX_LEN <= KEY_LEN);
            if PREFIX_LEN > 0 && PREFIX_LEN < KEY_LEN {
                assert!(
                    <O as KeyOrdering<KEY_LEN>>::Segmentation::SEGMENTS
                        [O::TREE_TO_KEY[PREFIX_LEN - 1]]
                        != <O as KeyOrdering<KEY_LEN>>::Segmentation::SEGMENTS
                            [O::TREE_TO_KEY[PREFIX_LEN]],
                    "PREFIX_LEN must align to segment boundary",
                );
            }
        }
        if let Some(root) = &self.root {
            root.segmented_len(0, prefix)
        } else {
            0
        }
    }

    /// Iterates over all keys in the PATCH.
    /// The keys are returned in key ordering but random order.
    pub fn iter<'a>(&'a self) -> PATCHIterator<'a, KEY_LEN, O> {
        PATCHIterator::new(self)
    }

    /// Iterates over all keys in the PATCH.
    /// The keys are returned in key ordering and tree order.
    pub fn iter_ordered<'a>(&'a self) -> PATCHOrderedIterator<'a, KEY_LEN, O> {
        PATCHOrderedIterator::new(self)
    }

    /// Iterate over all prefixes of the given length in the PATCH.
    /// The prefixes are naturally returned in tree ordering and tree order.
    /// A count of the number of elements for the given prefix is also returned.
    pub fn iter_prefix_count<'a, const PREFIX_LEN: usize>(
        &'a self,
    ) -> PATCHPrefixIterator<'a, KEY_LEN, PREFIX_LEN, O> {
        PATCHPrefixIterator::new(self)
    }

    /// Unions this PATCH with another PATCH.
    ///
    /// The other PATCH is consumed, and this PATCH is updated in place.
    pub fn union(&mut self, other: Self) {
        if let Some(other) = other.root {
            if self.root.is_some() {
                Head::union(&mut self.root, other, 0);
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

    /// Calculates the average fill level for branch nodes grouped by their
    /// branching factor. The returned array contains eight entries for branch
    /// sizes `2`, `4`, `8`, `16`, `32`, `64`, `128` and `256` in that order.
    #[cfg(debug_assertions)]
    pub fn debug_branch_fill(&self) -> [f32; 8] {
        let mut counts = [0u64; 8];
        let mut used = [0u64; 8];

        if let Some(root) = &self.root {
            let mut stack = Vec::new();
            stack.push(root);

            while let Some(head) = stack.pop() {
                match head.body() {
                    BodyPtr::Leaf(_) => {}
                    BodyPtr::Branch(branch) => unsafe {
                        let b = branch.as_ref();
                        let size = b.child_table.len();
                        let idx = size.trailing_zeros() as usize - 1;
                        counts[idx] += 1;
                        used[idx] += b.child_table.iter().filter(|c| c.is_some()).count() as u64;
                        for child in b.child_table.iter().filter_map(|c| c.as_ref()) {
                            stack.push(child);
                        }
                    },
                }
            }
        }

        let mut avg = [0f32; 8];
        for i in 0..8 {
            if counts[i] > 0 {
                let size = 1u64 << (i + 1);
                avg[i] = used[i] as f32 / (counts[i] as f32 * size as f32);
            }
        }
        avg
    }
}

impl<const KEY_LEN: usize, O> PartialEq for PATCH<KEY_LEN, O>
where
    O: KeyOrdering<KEY_LEN>,
{
    fn eq(&self, other: &Self) -> bool {
        self.root.as_ref().map(|root| root.hash()) == other.root.as_ref().map(|root| root.hash())
    }
}

impl<const KEY_LEN: usize, O> Eq for PATCH<KEY_LEN, O> where O: KeyOrdering<KEY_LEN> {}

impl<'a, const KEY_LEN: usize, O> IntoIterator for &'a PATCH<KEY_LEN, O>
where
    O: KeyOrdering<KEY_LEN>,
{
    type Item = &'a [u8; KEY_LEN];
    type IntoIter = PATCHIterator<'a, KEY_LEN, O>;

    fn into_iter(self) -> Self::IntoIter {
        PATCHIterator::new(self)
    }
}

/// An iterator over all keys in a PATCH.
/// The keys are returned in key ordering but in random order.
pub struct PATCHIterator<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> {
    stack: ArrayVec<std::slice::Iter<'a, Option<Head<KEY_LEN, O>>>, KEY_LEN>,
    remaining: usize,
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> PATCHIterator<'a, KEY_LEN, O> {
    fn new(patch: &'a PATCH<KEY_LEN, O>) -> Self {
        let mut r = PATCHIterator {
            stack: ArrayVec::new(),
            remaining: patch.len().min(usize::MAX as u64) as usize,
        };
        r.stack.push(std::slice::from_ref(&patch.root).iter());
        r
    }
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> Iterator for PATCHIterator<'a, KEY_LEN, O> {
    type Item = &'a [u8; KEY_LEN];

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.stack.last_mut()?;
        loop {
            if let Some(child) = iter.next() {
                if let Some(child) = child {
                    match child.body() {
                        BodyPtr::Leaf(leaf) => unsafe {
                            self.remaining = self.remaining.saturating_sub(1);
                            return Some(&leaf.as_ref().key);
                        },
                        BodyPtr::Branch(branch) => unsafe {
                            self.stack.push(branch.as_ref().child_table.iter());
                            iter = self.stack.last_mut()?;
                        },
                    }
                }
            } else {
                self.stack.pop();
                iter = self.stack.last_mut()?;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> ExactSizeIterator
    for PATCHIterator<'a, KEY_LEN, O>
{
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> std::iter::FusedIterator
    for PATCHIterator<'a, KEY_LEN, O>
{
}

/// An iterator over all keys in a PATCH that have a given prefix.
/// The keys are returned in tree ordering and in tree order.
pub struct PATCHOrderedIterator<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> {
    stack: Vec<ArrayVec<&'a Head<KEY_LEN, O>, 256>>,
    remaining: usize,
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> PATCHOrderedIterator<'a, KEY_LEN, O> {
    fn new(patch: &'a PATCH<KEY_LEN, O>) -> Self {
        let mut r = PATCHOrderedIterator {
            stack: Vec::with_capacity(KEY_LEN),
            remaining: patch.len().min(usize::MAX as u64) as usize,
        };
        if let Some(root) = &patch.root {
            r.stack.push(ArrayVec::new());
            match root.body() {
                BodyPtr::Leaf(_) => {
                    r.stack[0].push(root);
                }
                BodyPtr::Branch(branch) => unsafe {
                    let first_level = &mut r.stack[0];
                    first_level.extend(
                        branch
                            .as_ref()
                            .child_table
                            .iter()
                            .filter_map(|c| c.as_ref()),
                    );
                    first_level.sort_unstable_by_key(|&k| Reverse(k.key())); // We need to reverse here because we pop from the vec.
                },
            }
        }
        r
    }
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> Iterator
    for PATCHOrderedIterator<'a, KEY_LEN, O>
{
    type Item = &'a [u8; KEY_LEN];

    fn next(&mut self) -> Option<Self::Item> {
        let mut level = self.stack.last_mut()?;
        loop {
            if let Some(child) = level.pop() {
                match child.body() {
                    BodyPtr::Leaf(leaf) => unsafe {
                        self.remaining = self.remaining.saturating_sub(1);
                        return Some(&leaf.as_ref().key);
                    },
                    BodyPtr::Branch(branch) => unsafe {
                        self.stack.push(ArrayVec::new());
                        level = self.stack.last_mut()?;
                        level.extend(
                            branch
                                .as_ref()
                                .child_table
                                .iter()
                                .filter_map(|c| c.as_ref()),
                        );
                        level.sort_unstable_by_key(|&k| Reverse(k.key())); // We need to reverse here because we pop from the vec.
                    },
                }
            } else {
                self.stack.pop();
                level = self.stack.last_mut()?;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> ExactSizeIterator
    for PATCHOrderedIterator<'a, KEY_LEN, O>
{
}

impl<'a, const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>> std::iter::FusedIterator
    for PATCHOrderedIterator<'a, KEY_LEN, O>
{
}

/// An iterator over all keys in a PATCH that have a given prefix.
/// The keys are returned in tree ordering and in tree order.
pub struct PATCHPrefixIterator<
    'a,
    const KEY_LEN: usize,
    const PREFIX_LEN: usize,
    O: KeyOrdering<KEY_LEN>,
> {
    stack: Vec<ArrayVec<&'a Head<KEY_LEN, O>, 256>>,
}

impl<'a, const KEY_LEN: usize, const PREFIX_LEN: usize, O: KeyOrdering<KEY_LEN>>
    PATCHPrefixIterator<'a, KEY_LEN, PREFIX_LEN, O>
{
    fn new(patch: &'a PATCH<KEY_LEN, O>) -> Self {
        const {
            assert!(PREFIX_LEN <= KEY_LEN);
        }
        let mut r = PATCHPrefixIterator {
            stack: Vec::with_capacity(PREFIX_LEN),
        };
        if let Some(root) = &patch.root {
            r.stack.push(ArrayVec::new());
            if root.end_depth() >= PREFIX_LEN {
                r.stack[0].push(root);
            } else {
                let BodyPtr::Branch(branch) = root.body() else {
                    unreachable!();
                };
                unsafe {
                    let first_level = &mut r.stack[0];
                    first_level.extend(
                        branch
                            .as_ref()
                            .child_table
                            .iter()
                            .filter_map(|c| c.as_ref()),
                    );
                    first_level.sort_unstable_by_key(|&k| Reverse(k.key())); // We need to reverse here because we pop from the vec.
                }
            }
        }
        r
    }
}

impl<'a, const KEY_LEN: usize, const PREFIX_LEN: usize, O: KeyOrdering<KEY_LEN>> Iterator
    for PATCHPrefixIterator<'a, KEY_LEN, PREFIX_LEN, O>
{
    type Item = ([u8; PREFIX_LEN], u64);

    fn next(&mut self) -> Option<Self::Item> {
        let mut level = self.stack.last_mut()?;
        loop {
            if let Some(child) = level.pop() {
                if child.end_depth() >= PREFIX_LEN {
                    let key = O::tree_ordered(child.childleaf_key());
                    let suffix_count = child.count();
                    return Some((key[0..PREFIX_LEN].try_into().unwrap(), suffix_count));
                } else {
                    let BodyPtr::Branch(branch) = child.body() else {
                        unreachable!();
                    };
                    unsafe {
                        self.stack.push(ArrayVec::new());
                        level = self.stack.last_mut()?;
                        level.extend(
                            branch
                                .as_ref()
                                .child_table
                                .iter()
                                .filter_map(|c| c.as_ref()),
                        );
                        level.sort_unstable_by_key(|&k| Reverse(k.key())); // We need to reverse here because we pop from the vec.
                    }
                }
            } else {
                self.stack.pop();
                level = self.stack.last_mut()?;
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
        let head = Head::<64, IdentityOrder>::new::<Leaf<64>>(0, NonNull::dangling());
        assert_eq!(head.tag(), HeadTag::Leaf);
        mem::forget(head);
    }

    #[test]
    fn head_key() {
        for k in 0..=255 {
            let head = Head::<64, IdentityOrder>::new::<Leaf<64>>(k, NonNull::dangling());
            assert_eq!(head.key(), k);
            mem::forget(head);
        }
    }

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head<64, IdentityOrder>>(), 8);
    }

    #[test]
    fn empty_tree() {
        let _tree = PATCH::<64, IdentityOrder>::new();
    }

    #[test]
    fn tree_put_one() {
        const KEY_SIZE: usize = 64;
        let mut tree = PATCH::<KEY_SIZE, IdentityOrder>::new();
        let entry = Entry::new(&[0; KEY_SIZE]);
        tree.insert(&entry);
    }

    #[test]
    fn tree_put_same() {
        const KEY_SIZE: usize = 64;
        let mut tree = PATCH::<KEY_SIZE, IdentityOrder>::new();
        let entry = Entry::new(&[0; KEY_SIZE]);
        tree.insert(&entry);
        tree.insert(&entry);
    }

    #[test]
    fn branch_size() {
        assert_eq!(
            mem::size_of::<Branch<64, IdentityOrder, [Option<Head<64, IdentityOrder>>; 2]>>(),
            64
        );
        assert_eq!(
            mem::size_of::<Branch<64, IdentityOrder, [Option<Head<64, IdentityOrder>>; 4]>>(),
            48 + 16 * 2
        );
        assert_eq!(
            mem::size_of::<Branch<64, IdentityOrder, [Option<Head<64, IdentityOrder>>; 8]>>(),
            48 + 16 * 4
        );
        assert_eq!(
            mem::size_of::<Branch<64, IdentityOrder, [Option<Head<64, IdentityOrder>>; 16]>>(),
            48 + 16 * 8
        );
        assert_eq!(
            mem::size_of::<Branch<64, IdentityOrder, [Option<Head<32, IdentityOrder>>; 32]>>(),
            48 + 16 * 16
        );
        assert_eq!(
            mem::size_of::<Branch<64, IdentityOrder, [Option<Head<64, IdentityOrder>>; 64]>>(),
            48 + 16 * 32
        );
        assert_eq!(
            mem::size_of::<Branch<64, IdentityOrder, [Option<Head<64, IdentityOrder>>; 128]>>(),
            48 + 16 * 64
        );
        assert_eq!(
            mem::size_of::<Branch<64, IdentityOrder, [Option<Head<64, IdentityOrder>>; 256]>>(),
            48 + 16 * 128
        );
    }

    /// Checks what happens if we join two PATCHes that
    /// only contain a single element each, that differs in the last byte.
    #[test]
    fn tree_union_single() {
        const KEY_SIZE: usize = 8;
        let mut left = PATCH::<KEY_SIZE, IdentityOrder>::new();
        let mut right = PATCH::<KEY_SIZE, IdentityOrder>::new();
        let left_entry = Entry::new(&[0, 0, 0, 0, 0, 0, 0, 0]);
        let right_entry = Entry::new(&[0, 0, 0, 0, 0, 0, 0, 1]);
        left.insert(&left_entry);
        right.insert(&right_entry);
        left.union(right);
        assert_eq!(left.len(), 2);
    }

    proptest! {
        #[test]
        fn tree_insert(keys in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 1..1024)) {
            let mut tree = PATCH::<64, IdentityOrder>::new();
            for key in keys {
                let key: [u8; 64] = key.try_into().unwrap();
                let entry = Entry::new(&key);
                tree.insert(&entry);
            }
        }

        #[test]
        fn tree_len(keys in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 1..1024)) {
            let mut tree = PATCH::<64, IdentityOrder>::new();
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
        fn tree_infixes(keys in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 1..1024)) {
            let mut tree = PATCH::<64, IdentityOrder>::new();
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
        fn tree_iter(keys in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 1..1024)) {
            let mut tree = PATCH::<64, IdentityOrder>::new();
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
        fn tree_union(left in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 200),
                        right in prop::collection::vec(prop::collection::vec(0u8..=255, 64), 200)) {
            let mut set = HashSet::new();

            let mut left_tree = PATCH::<64, IdentityOrder>::new();
            for entry in left {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                let entry = Entry::new(&key);
                left_tree.insert(&entry);
                set.insert(key);
            }

            let mut right_tree = PATCH::<64, IdentityOrder>::new();
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

            let mut left_tree = PATCH::<64, IdentityOrder>::new();
            for entry in left {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                let entry = Entry::new(&key);
                left_tree.insert(&entry);
                set.insert(key);
            }

            let right_tree = PATCH::<64, IdentityOrder>::new();

            left_tree.union(right_tree);

            let mut set_vec = Vec::from_iter(set.into_iter());
            let mut tree_vec = vec![];
            left_tree.infixes(&[0; 0], &mut |&x: &[u8;64]| tree_vec.push(x));

            set_vec.sort();
            tree_vec.sort();

            prop_assert_eq!(set_vec, tree_vec);
            }

        // I got a feeling that we're not testing COW properly.
        // We should check if a tree remains the same after a clone of it
        // is modified by inserting new keys.

        #[test]
        fn cow_on_insert(base_keys in prop::collection::vec(prop::collection::vec(0u8..=255, 8), 1..1024),
                         new_keys in prop::collection::vec(prop::collection::vec(0u8..=255, 8), 1..1024)) {
            // Note that we can't compare the trees directly, as that uses the hash,
            // which might not be affected by nodes in lower levels being changed accidentally.
            // Instead we need to iterate over the keys and check if they are the same.

            let mut tree = PATCH::<8, IdentityOrder>::new();
            for key in base_keys {
                let key: [u8; 8] = key[..].try_into().unwrap();
                let entry = Entry::new(&key);
                tree.insert(&entry);
            }
            let base_tree_content: Vec<[u8; 8]> = tree.iter().copied().collect();

            let mut tree_clone = tree.clone();
            for key in new_keys {
                let key: [u8; 8] = key[..].try_into().unwrap();
                let entry = Entry::new(&key);
                tree_clone.insert(&entry);
            }

            let new_tree_content: Vec<[u8; 8]> = tree.iter().copied().collect();
            prop_assert_eq!(base_tree_content, new_tree_content);
        }

        #[test]
        fn cow_on_union(base_keys in prop::collection::vec(prop::collection::vec(0u8..=255, 8), 1..1024),
                         new_keys in prop::collection::vec(prop::collection::vec(0u8..=255, 8), 1..1024)) {
            // Note that we can't compare the trees directly, as that uses the hash,
            // which might not be affected by nodes in lower levels being changed accidentally.
            // Instead we need to iterate over the keys and check if they are the same.

            let mut tree = PATCH::<8, IdentityOrder>::new();
            for key in base_keys {
                let key: [u8; 8] = key[..].try_into().unwrap();
                let entry = Entry::new(&key);
                tree.insert(&entry);
            }
            let base_tree_content: Vec<[u8; 8]> = tree.iter().copied().collect();

            let mut tree_clone = tree.clone();
            let mut new_tree = PATCH::<8, IdentityOrder>::new();
            for key in new_keys {
                let key: [u8; 8] = key[..].try_into().unwrap();
                let entry = Entry::new(&key);
                new_tree.insert(&entry);
            }
            tree_clone.union(new_tree);

            let new_tree_content: Vec<[u8; 8]> = tree.iter().copied().collect();
            prop_assert_eq!(base_tree_content, new_tree_content);
        }
    }
}
