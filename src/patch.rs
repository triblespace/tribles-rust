// Persistent Adaptive Trie with Cuckoos and Hashes
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
use core::hash::Hasher;
use rand::thread_rng;
use rand::RngCore;
use std::cmp::Reverse;
use std::convert::TryInto;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::transmute;
use std::sync::Once;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("compilation is only possible for 64-bit targets");

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

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum Body<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    Leaf(*mut Leaf<KEY_LEN>),
    Branch(*mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>),
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
    pub(crate) unsafe fn new<T>(tag: HeadTag, key: u8, ptr: *mut T) -> Self {
        Self {
            tptr: std::ptr::NonNull::new(ptr.map_addr(|addr| {
                ((addr as u64 & 0x00_00_ff_ff_ff_ff_ff_ffu64)
                    | ((key as u64) << 48)
                    | ((tag as u64) << 56)) as usize
            }) as *mut u8)
            .unwrap(),
            key_ordering: PhantomData,
            key_segments: PhantomData,
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
    pub(crate) unsafe fn ptr<T>(&self) -> *mut T {
        self.tptr
            .as_ptr()
            .map_addr(|addr| ((((addr as u64) << 16) as i64) >> 16) as usize) as *mut T
    }

    pub(crate) fn body(&self) -> Body<KEY_LEN, O, S> {
        unsafe {
            match self.tag() {
                HeadTag::Leaf => Body::Leaf(self.ptr()),
                HeadTag::Branch2 => {
                    Body::Branch(
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>>(),
                    )
                }
                HeadTag::Branch4 => {
                    Body::Branch(
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>>(),
                    )
                }
                HeadTag::Branch8 => {
                    Body::Branch(
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>>(),
                    )
                }
                HeadTag::Branch16 => {
                    Body::Branch(
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>>(),
                    )
                }
                HeadTag::Branch32 => {
                    Body::Branch(
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>>(),
                    )
                }
                HeadTag::Branch64 => {
                    Body::Branch(
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>>(),
                    )
                }
                HeadTag::Branch128 => {
                    Body::Branch(
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>>(),
                    )
                }
                HeadTag::Branch256 => {
                    Body::Branch(
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 256]>>(),
                    )
                }
            }
        }
    }

    pub(crate) fn with_start(mut self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
        let leaf_key = self.leaf_key();
        let i = O::key_index(new_start_depth);
        let key = leaf_key[i];
        self.set_key(key);
        self
    }

    pub(crate) fn cow(&mut self) {
        unsafe {
            match self.tag() {
                HeadTag::Leaf => panic!("called cow on leaf"),
                HeadTag::Branch2 => {
                    let branch =
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>>();
                    if let Some(copy) = Branch::rc_cow(branch) {
                        *self = Head::new(self.tag(), self.key(), copy);
                    }
                }
                HeadTag::Branch4 => {
                    let branch =
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>>();
                    if let Some(copy) = Branch::rc_cow(branch) {
                        *self = Head::new(self.tag(), self.key(), copy);
                    }
                }
                HeadTag::Branch8 => {
                    let branch =
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>>();
                    if let Some(copy) = Branch::rc_cow(branch) {
                        *self = Head::new(self.tag(), self.key(), copy);
                    }
                }
                HeadTag::Branch16 => {
                    let branch =
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>>();
                    if let Some(copy) = Branch::rc_cow(branch) {
                        *self = Head::new(self.tag(), self.key(), copy);
                    }
                }
                HeadTag::Branch32 => {
                    let branch =
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>>();
                    if let Some(copy) = Branch::rc_cow(branch) {
                        *self = Head::new(self.tag(), self.key(), copy);
                    }
                }
                HeadTag::Branch64 => {
                    let branch =
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>>();
                    if let Some(copy) = Branch::rc_cow(branch) {
                        *self = Head::new(self.tag(), self.key(), copy);
                    }
                }
                HeadTag::Branch128 => {
                    let branch =
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>>();
                    if let Some(copy) = Branch::rc_cow(branch) {
                        *self = Head::new(self.tag(), self.key(), copy);
                    }
                }
                HeadTag::Branch256 => {
                    let branch =
                        self.ptr::<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 256]>>();
                    if let Some(copy) = Branch::rc_cow(branch) {
                        *self = Head::new(self.tag(), self.key(), copy);
                    }
                }
            }
        }
    }

    pub(crate) fn insert_child(&mut self, mut child: Head<KEY_LEN, O, S>) -> Option<()> {
        unsafe {
            if self.tag() == HeadTag::Branch2 {
                let branch = self.ptr::<Branch2<KEY_LEN, O, S>>();
                child = (*branch).child_table.table_insert(child)?;
                let grown = Branch2::<KEY_LEN, O, S>::grow(branch);
                *self = Head::new(HeadTag::Branch4, self.key(), grown);
            }
            if self.tag() == HeadTag::Branch4 {
                let branch = self.ptr::<Branch4<KEY_LEN, O, S>>();
                child = (*branch).child_table.table_insert(child)?;
                let grown = Branch4::<KEY_LEN, O, S>::grow(branch);
                *self = Head::new(HeadTag::Branch8, self.key(), grown);
            }
            if self.tag() == HeadTag::Branch8 {
                let branch = self.ptr::<Branch8<KEY_LEN, O, S>>();
                child = (*branch).child_table.table_insert(child)?;
                let grown = Branch8::<KEY_LEN, O, S>::grow(branch);
                *self = Head::new(HeadTag::Branch16, self.key(), grown);
            }
            if self.tag() == HeadTag::Branch16 {
                let branch = self.ptr::<Branch16<KEY_LEN, O, S>>();
                child = (*branch).child_table.table_insert(child)?;
                let grown = Branch16::<KEY_LEN, O, S>::grow(branch);
                *self = Head::new(HeadTag::Branch32, self.key(), grown);
            }
            if self.tag() == HeadTag::Branch32 {
                let branch = self.ptr::<Branch32<KEY_LEN, O, S>>();
                child = (*branch).child_table.table_insert(child)?;
                let grown = Branch32::<KEY_LEN, O, S>::grow(branch);
                *self = Head::new(HeadTag::Branch64, self.key(), grown);
            }
            if self.tag() == HeadTag::Branch64 {
                let branch = self.ptr::<Branch64<KEY_LEN, O, S>>();
                child = (*branch).child_table.table_insert(child)?;
                let grown = Branch64::<KEY_LEN, O, S>::grow(branch);
                *self = Head::new(HeadTag::Branch128, self.key(), grown);
            }
            if self.tag() == HeadTag::Branch128 {
                let branch = self.ptr::<Branch128<KEY_LEN, O, S>>();
                child = (*branch).child_table.table_insert(child)?;
                let grown = Branch128::<KEY_LEN, O, S>::grow(branch);
                *self = Head::new(HeadTag::Branch256, self.key(), grown);
            }
            if self.tag() == HeadTag::Branch256 {
                let branch = self.ptr::<Branch256<KEY_LEN, O, S>>();
                _ = (*branch).child_table.table_insert(child)?;
                panic!("failed to insert on Branch256");
            }

            panic!("failed to insert on non branch");
        }
    }

    pub unsafe fn upsert<F>(&mut self, inserted: Head<KEY_LEN, O, S>, update: F)
    where
        F: Fn(&mut Head<KEY_LEN, O, S>, Head<KEY_LEN, O, S>),
    {
        self.cow();
        match self.body() {
            Body::Leaf(_) => panic!("upsert on leaf"),
            Body::Branch(branch) => {
                let inserted = inserted.with_start((*branch).end_depth as usize);
                let key = inserted.key();
                if let Some(child) = (*branch).child_table.table_get_mut(key) {
                    let old_child_hash = child.hash();
                    let old_child_segment_count = child.count_segment((*branch).end_depth as usize);
                    let old_child_leaf_count = child.count();

                    update(child, inserted);

                    (*branch).hash = ((*branch).hash ^ old_child_hash) ^ child.hash();
                    (*branch).segment_count = ((*branch).segment_count - old_child_segment_count)
                        + child.count_segment((*branch).end_depth as usize);
                    (*branch).leaf_count =
                        ((*branch).leaf_count - old_child_leaf_count) + child.count();
                } else {
                    let end_depth = (*branch).end_depth as usize;
                    (*branch).leaf_count += inserted.count();
                    (*branch).segment_count += inserted.count_segment(end_depth);
                    (*branch).hash ^= inserted.hash();

                    self.insert_child(inserted);
                }
            }
        }
    }

    pub(crate) fn count(&self) -> u64 {
        unsafe {
            match self.body() {
                Body::Leaf(_) => 1,
                Body::Branch(branch) => (*branch).leaf_count,
            }
        }
    }

    pub(crate) fn count_segment(&self, at_depth: usize) -> u64 {
        match self.body() {
            Body::Leaf(_) => 1,
            Body::Branch(branch) => {
                branch::Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::count_segment(
                    branch, at_depth,
                )
            }
        }
    }

    pub(crate) fn hash(&self) -> u128 {
        unsafe {
            match self.body() {
                Body::Leaf(leaf) => Leaf::<KEY_LEN>::hash(leaf),
                Body::Branch(branch) => (*branch).hash,
            }
        }
    }

    pub(crate) fn end_depth(&self) -> usize {
        unsafe {
            match self.body() {
                Body::Leaf(_) => KEY_LEN as usize,
                Body::Branch(branch) => (*branch).end_depth as usize,
            }
        }
    }

    pub(crate) unsafe fn childleaf(&self) -> *const Leaf<KEY_LEN> {
        match self.body() {
            Body::Leaf(leaf) => leaf,
            Body::Branch(branch) => (*branch).childleaf,
        }
    }

    pub(crate) fn leaf_key<'a>(&'a self) -> &'a [u8; KEY_LEN] {
        unsafe {
            match self.body() {
                Body::Leaf(leaf) => &(*leaf).key,
                Body::Branch(branch) => &(*(*branch).childleaf).key,
            }
        }
    }

    pub(crate) fn insert_leaf(&mut self, leaf: Self, start_depth: usize) {
        unsafe {
            let head_depth = self.end_depth();
            let head_key = self.leaf_key();
            let leaf_key = leaf.leaf_key();

            let end_depth = std::cmp::min(head_depth, KEY_LEN);
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
                    child.insert_leaf(inserted, head_depth)
                });
            }
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
        unsafe {
            match self.body() {
                Body::Leaf(leaf) => Leaf::<KEY_LEN>::infixes::<PREFIX_LEN, INFIX_LEN, O, S, F>(
                    leaf, prefix, at_depth, f,
                ),
                Body::Branch(branch) => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::infixes(
                        branch, prefix, at_depth, f,
                    )
                }
            }
        }
    }

    pub(crate) fn has_prefix<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        unsafe {
            match self.body() {
                Body::Leaf(leaf) => {
                    Leaf::<KEY_LEN>::has_prefix::<O, PREFIX_LEN>(leaf, at_depth, prefix)
                }
                Body::Branch(branch) => {
                    branch::Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::has_prefix(
                        branch, at_depth, prefix,
                    )
                }
            }
        }
    }

    pub(crate) fn segmented_len<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        unsafe {
            match self.body() {
                Body::Leaf(leaf) => {
                    leaf::Leaf::<KEY_LEN>::segmented_len::<O, PREFIX_LEN>(leaf, at_depth, prefix)
                }
                Body::Branch(branch) => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::segmented_len(
                        branch, at_depth, prefix,
                    )
                }
            }
        }
    }

    pub(crate) fn union(&mut self, other: Self, at_depth: usize) {
        let self_hash = self.hash();
        let other_hash = other.hash();
        if self_hash == other_hash {
            return;
        }
        let self_depth = self.end_depth();
        let other_depth = other.end_depth();

        let self_key = self.leaf_key();
        let other_key = other.leaf_key();
        unsafe {
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

            other.take_or_clone_children(|other_child| {
                self.upsert(other_child, |child, inserted| {
                    child.union(inserted, self_depth)
                });
            });
        }
    }

    pub(crate) fn take_or_clone_children<F>(&self, f: F)
    where
        F: FnMut(Self),
    {
        unsafe {
            match self.body() {
                Body::Leaf(_) => panic!("take_or_clone_children on leaf"),
                Body::Branch(branch) => {
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::take_or_clone_children(
                        branch, f,
                    )
                }
            }
        }
    }

    pub(crate) fn iter_children(&self) -> std::slice::Iter<Option<Head<KEY_LEN, O, S>>> {
        unsafe {
            match self.body() {
                Body::Leaf(_) => panic!("iter_children on leaf"),
                Body::Branch(branch) => (&(*branch).child_table).iter(),
            }
        }
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
                    Self::new(self.tag(), self.key(), Leaf::<KEY_LEN>::rc_inc(self.ptr()))
                }
                HeadTag::Branch2 => Self::new(
                    self.tag(),
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch4 => Self::new(
                    self.tag(),
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch8 => Self::new(
                    self.tag(),
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch16 => Self::new(
                    self.tag(),
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch32 => Self::new(
                    self.tag(),
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch64 => Self::new(
                    self.tag(),
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch128 => Self::new(
                    self.tag(),
                    self.key(),
                    Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch256 => Self::new(
                    self.tag(),
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

#[derive(Debug, Clone)]
pub struct PATCH<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    root: Option<Head<KEY_LEN, O, S>>,
}

impl<const KEY_LEN: usize, O, S> PATCH<KEY_LEN, O, S>
where
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
{
    pub fn new() -> Self {
        init();
        PATCH { root: None }
    }

    pub fn insert(&mut self, entry: &Entry<KEY_LEN>) {
        if let Some(root) = &mut self.root {
            root.insert_leaf(entry.leaf(), 0);
        } else {
            self.root.replace(entry.leaf());
        }
    }

    pub fn len(&self) -> u64 {
        if let Some(root) = &self.root {
            root.count()
        } else {
            0
        }
    }

    pub fn infixes<const PREFIX_LEN: usize, const INFIX_LEN: usize, F>(
        &self,
        prefix: &[u8; PREFIX_LEN],
        mut f: F,
    ) where
        F: FnMut(&[u8; INFIX_LEN]), //TODO can we make this a ref too?
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
            root.infixes(prefix, 0, &mut f);
        }
    }

    pub fn has_prefix<const PREFIX_LEN: usize>(&self, prefix: &[u8; PREFIX_LEN]) -> bool {
        if let Some(root) = &self.root {
            root.has_prefix(0, prefix)
        } else {
            PREFIX_LEN == 0
        }
    }

    pub fn segmented_len<const PREFIX_LEN: usize>(&self, prefix: &[u8; PREFIX_LEN]) -> u64 {
        if let Some(root) = &self.root {
            root.segmented_len(0, prefix)
        } else {
            0
        }
    }

    pub fn iter_prefix<'a, const PREFIX_LEN: usize>(
        &'a self,
    ) -> PATCHPrefixIterator<'a, KEY_LEN, PREFIX_LEN, O, S> {
        PATCHPrefixIterator::new(self)
    }

    pub fn union(&mut self, other: Self) {
        if let Some(other) = other.root {
            if let Some(root) = &mut self.root {
                root.union(other, 0);
            } else {
                self.root.replace(other);
            }
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
    type Item = [u8; KEY_LEN];
    type IntoIter = PATCHIterator<'a, KEY_LEN, O, S>;

    fn into_iter(self) -> Self::IntoIter {
        PATCHIterator::new(self)
    }
}

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
    type Item = [u8; KEY_LEN];

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.stack.pop()?;
        loop {
            if let Some(child) = iter.next() {
                if let Some(child) = child {
                    match child.tag() {
                        HeadTag::Leaf => {
                            let leaf: *const Leaf<KEY_LEN> = unsafe { child.ptr() };
                            let key = O::tree_ordered(unsafe { &(*leaf).key });
                            self.stack.push(iter);
                            return Some(key);
                        }
                        _ => {
                            self.stack.push(iter);
                            iter = child.iter_children();
                        }
                    }
                }
            } else {
                iter = self.stack.pop()?;
            }
        }
    }
}

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
                    let leaf: *const Leaf<KEY_LEN> = unsafe { child.childleaf() };
                    let key = O::tree_ordered(unsafe { &(*leaf).key });
                    let suffix_count = child.count();
                    self.stack.push(level);
                    return Some((key[0..PREFIX_LEN].try_into().unwrap(), suffix_count));
                } else {
                    self.stack.push(level);
                    level = child.iter_children().filter_map(|c| c.as_ref()).collect();
                    level.sort_by_key(|&k| Reverse(k.key())); // We need to reverse here because we pop from the vec.
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
        let head = unsafe {
            Head::<64, IdentityOrder, SingleSegmentation>::new::<Leaf<64>>(
                HeadTag::Leaf,
                0,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(head.tag(), HeadTag::Leaf);
        mem::forget(head);
    }

    #[test]
    fn head_key() {
        for k in 0..=255 {
            let head = unsafe {
                Head::<64, IdentityOrder, SingleSegmentation>::new::<Leaf<64>>(
                    HeadTag::Leaf,
                    k,
                    std::ptr::null_mut(),
                )
            };
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
            tree_vec.push(key);
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
