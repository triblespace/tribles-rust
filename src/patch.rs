// Persistent Adaptive Trie with Cuckoos and Hashes

mod branch;
mod bytecursor;
mod entry;
mod leaf;
mod paddingcursor;

use branch::*;
pub use entry::Entry;
use leaf::*;

use crate::bytetable;
use crate::bytetable::*;
use core::hash::Hasher;
use rand::thread_rng;
use rand::RngCore;
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
    Empty = 0,
    Leaf,
    Branch2,
    Branch4,
    Branch8,
    Branch16,
    Branch32,
    Branch64,
    Branch128,
    Branch256,
}

#[repr(C)]
pub(crate) struct Head<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    tptr: *mut u8,
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
    pub(crate) fn empty() -> Self {
        Self {
            tptr: std::ptr::null_mut(),
            key_ordering: PhantomData,
            key_segments: PhantomData,
        }
    }

    pub(crate) unsafe fn new<T>(tag: HeadTag, key: u8, ptr: *mut T) -> Self {
        Self {
            tptr: ((ptr as u64 & 0x00_00_ff_ff_ff_ff_ff_ffu64)
                | ((key as u64) << 48)
                | ((tag as u64) << 56)) as *mut u8,
            key_ordering: PhantomData,
            key_segments: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn tag(&self) -> HeadTag {
        unsafe { transmute((self.tptr as u64 >> 56) as u8) }
    }

    #[inline]
    pub(crate) fn key(&self) -> Option<u8> {
        if self.tag() == HeadTag::Empty {
            None
        } else {
            Some((self.tptr as u64 >> 48) as u8)
        }
    }

    #[inline]
    pub(crate) fn set_key(&mut self, key: u8) {
        self.tptr =
            ((self.tptr as u64 & 0xff_00_ff_ff_ff_ff_ff_ffu64) | ((key as u64) << 48)) as *mut u8;
    }

    #[inline]
    pub(crate) unsafe fn ptr<T>(&self) -> *mut T {
        ((((self.tptr as u64) << 16) as i64) >> 16 as u64) as *mut T
    }

    // Node
    pub(crate) fn count(&self) -> u64 {
        unsafe {
            match self.tag() {
                HeadTag::Empty => 0,
                HeadTag::Leaf => 1,
                HeadTag::Branch2 => {
                    let node: *const Branch2<KEY_LEN, O, S> = self.ptr();
                    (*node).leaf_count
                }
                HeadTag::Branch4 => {
                    let node: *const Branch4<KEY_LEN, O, S> = self.ptr();
                    (*node).leaf_count
                }
                HeadTag::Branch8 => {
                    let node: *const Branch8<KEY_LEN, O, S> = self.ptr();
                    (*node).leaf_count
                }
                HeadTag::Branch16 => {
                    let node: *const Branch16<KEY_LEN, O, S> = self.ptr();
                    (*node).leaf_count
                }
                HeadTag::Branch32 => {
                    let node: *const Branch32<KEY_LEN, O, S> = self.ptr();
                    (*node).leaf_count
                }
                HeadTag::Branch64 => {
                    let node: *const Branch64<KEY_LEN, O, S> = self.ptr();
                    (*node).leaf_count
                }
                HeadTag::Branch128 => {
                    let node: *const Branch128<KEY_LEN, O, S> = self.ptr();
                    (*node).leaf_count
                }
                HeadTag::Branch256 => {
                    let node: *const Branch256<KEY_LEN, O, S> = self.ptr();
                    (*node).leaf_count
                }
            }
        }
    }

    pub(crate) fn count_segment(&self, at_depth: usize) -> u64 {
        unsafe {
            match self.tag() {
                HeadTag::Empty => 0,
                HeadTag::Leaf => 1,
                HeadTag::Branch2 => Branch2::<KEY_LEN, O, S>::count_segment(self.ptr(), at_depth),
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

    pub(crate) fn with_start(&self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
        match self.tag() {
            HeadTag::Empty => Self::empty(),
            _ => {
                let key = self.peek(new_start_depth);
                let mut clone = self.clone();
                clone.set_key(key);
                clone
            }
        }
    }

    pub(crate) fn peek(&self, at_depth: usize) -> u8 {
        let key_depth = O::key_index(at_depth);
        unsafe {
            match self.tag() {
                HeadTag::Empty => panic!("peeked on empty"),
                HeadTag::Leaf => Leaf::<KEY_LEN>::peek(self.ptr(), key_depth),
                HeadTag::Branch2 => Branch2::<KEY_LEN, O, S>::peek(self.ptr(), key_depth),
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::peek(self.ptr(), key_depth),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::peek(self.ptr(), key_depth),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::peek(self.ptr(), key_depth),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::peek(self.ptr(), key_depth),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::peek(self.ptr(), key_depth),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::peek(self.ptr(), key_depth),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::peek(self.ptr(), key_depth),
            }
        }
    }

    pub(crate) fn insert(&mut self, child: Self, hash: u128) {
        unsafe {
            let displaced = match self.tag() {
                HeadTag::Empty => panic!("insert on empty"),
                HeadTag::Leaf => panic!("insert on leaf"),
                HeadTag::Branch2 => Branch2::<KEY_LEN, O, S>::insert(self.ptr(), child, hash),
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::insert(self.ptr(), child, hash),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::insert(self.ptr(), child, hash),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::insert(self.ptr(), child, hash),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::insert(self.ptr(), child, hash),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::insert(self.ptr(), child, hash),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::insert(self.ptr(), child, hash),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::insert(self.ptr(), child, hash),
            };
            if displaced.key() != None {
                self.growing_reinsert(displaced);
            }
        }
    }

    pub(crate) fn growing_reinsert(&mut self, child: Self) {
        unsafe {
            let mut displaced = child;
            if self.tag() == HeadTag::Branch2 {
                Branch2::<KEY_LEN, O, S>::grow(self);
                let node: *mut Branch4<KEY_LEN, O, S> = self.ptr();
                displaced = (*node).child_table.put(displaced);
                if displaced.key() == None {
                    return;
                }
            }
            if self.tag() == HeadTag::Branch4 {
                Branch4::<KEY_LEN, O, S>::grow(self);
                let node: *mut Branch8<KEY_LEN, O, S> = self.ptr();
                displaced = (*node).child_table.put(displaced);
                if displaced.key() == None {
                    return;
                }
            }
            if self.tag() == HeadTag::Branch8 {
                Branch8::<KEY_LEN, O, S>::grow(self);
                let node: *mut Branch16<KEY_LEN, O, S> = self.ptr();
                displaced = (*node).child_table.put(displaced);
                if displaced.key() == None {
                    return;
                }
            }
            if self.tag() == HeadTag::Branch16 {
                Branch16::<KEY_LEN, O, S>::grow(self);
                let node: *mut Branch32<KEY_LEN, O, S> = self.ptr();
                displaced = (*node).child_table.put(displaced);
                if displaced.key() == None {
                    return;
                }
            }
            if self.tag() == HeadTag::Branch32 {
                Branch32::<KEY_LEN, O, S>::grow(self);
                let node: *mut Branch64<KEY_LEN, O, S> = self.ptr();
                displaced = (*node).child_table.put(displaced);
                if displaced.key() == None {
                    return;
                }
            }
            if self.tag() == HeadTag::Branch64 {
                Branch64::<KEY_LEN, O, S>::grow(self);
                let node: *mut Branch128<KEY_LEN, O, S> = self.ptr();
                displaced = (*node).child_table.put(displaced);
                if displaced.key() == None {
                    return;
                }
            }
            if self.tag() == HeadTag::Branch128 {
                Branch128::<KEY_LEN, O, S>::grow(self);
                let node: *mut Branch256<KEY_LEN, O, S> = self.ptr();
                displaced = (*node).child_table.put(displaced);
                if displaced.key() == None {
                    return;
                }
            }
            if self.tag() == HeadTag::Branch256 {
                panic!("failed to insert on Branch256");
            }
            panic!("failed to insert on non branch");
        }
    }

    pub(crate) fn hash(&self) -> u128 {
        unsafe {
            match self.tag() {
                HeadTag::Empty => 0,
                HeadTag::Leaf => Leaf::<KEY_LEN>::hash(self.ptr()),
                HeadTag::Branch2 => (*self.ptr::<Branch2<KEY_LEN, O, S>>()).hash,
                HeadTag::Branch4 => (*self.ptr::<Branch4<KEY_LEN, O, S>>()).hash,
                HeadTag::Branch8 => (*self.ptr::<Branch8<KEY_LEN, O, S>>()).hash,
                HeadTag::Branch16 => (*self.ptr::<Branch16<KEY_LEN, O, S>>()).hash,
                HeadTag::Branch32 => (*self.ptr::<Branch32<KEY_LEN, O, S>>()).hash,
                HeadTag::Branch64 => (*self.ptr::<Branch64<KEY_LEN, O, S>>()).hash,
                HeadTag::Branch128 => (*self.ptr::<Branch128<KEY_LEN, O, S>>()).hash,
                HeadTag::Branch256 => (*self.ptr::<Branch256<KEY_LEN, O, S>>()).hash,
            }
        }
    }

    pub(crate) fn end_depth(&self) -> usize {
        unsafe {
            match self.tag() {
                HeadTag::Empty => panic!("called end_depth on empty"),
                HeadTag::Leaf => KEY_LEN as usize,
                HeadTag::Branch2 => (*self.ptr::<Branch2<KEY_LEN, O, S>>()).end_depth as usize,
                HeadTag::Branch4 => (*self.ptr::<Branch4<KEY_LEN, O, S>>()).end_depth as usize,
                HeadTag::Branch8 => (*self.ptr::<Branch8<KEY_LEN, O, S>>()).end_depth as usize,
                HeadTag::Branch16 => (*self.ptr::<Branch16<KEY_LEN, O, S>>()).end_depth as usize,
                HeadTag::Branch32 => (*self.ptr::<Branch32<KEY_LEN, O, S>>()).end_depth as usize,
                HeadTag::Branch64 => (*self.ptr::<Branch64<KEY_LEN, O, S>>()).end_depth as usize,
                HeadTag::Branch128 => (*self.ptr::<Branch128<KEY_LEN, O, S>>()).end_depth as usize,
                HeadTag::Branch256 => (*self.ptr::<Branch256<KEY_LEN, O, S>>()).end_depth as usize,
            }
        }
    }

    //TODO rename
    pub(crate) unsafe fn min(&self) -> *const Leaf<KEY_LEN> {
        unsafe {
            match self.tag() {
                HeadTag::Empty => std::ptr::null_mut(),
                HeadTag::Leaf => self.ptr::<Leaf<KEY_LEN>>(),
                HeadTag::Branch2 => (*self.ptr::<Branch2<KEY_LEN, O, S>>()).min,
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

    pub(crate) fn put(&mut self, entry: &Entry<KEY_LEN>, start_depth: usize) {
        unsafe {
            match self.tag() {
                HeadTag::Empty => {
                    *self = entry.leaf(start_depth);
                }
                HeadTag::Leaf => Leaf::<KEY_LEN>::put(self, entry, start_depth),
                HeadTag::Branch2 => Branch2::<KEY_LEN, O, S>::put(self, entry, start_depth),
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::put(self, entry, start_depth),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::put(self, entry, start_depth),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::put(self, entry, start_depth),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::put(self, entry, start_depth),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::put(self, entry, start_depth),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::put(self, entry, start_depth),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::put(self, entry, start_depth),
            }
        }
    }

    pub(crate) fn each_child<F>(&self, mut f: F)
    where
        F: FnMut(u8, &Self),
    {
        unsafe {
            match self.tag() {
                HeadTag::Empty => panic!("called `each_child` on Empty"),
                HeadTag::Leaf => panic!("called `each_child` on Leaf"),
                HeadTag::Branch2 => {
                    let node: *mut Branch2<KEY_LEN, O, S> = self.ptr();
                    for bucket in &(*node).child_table.buckets {
                        // TODO replace this with iterator
                        for child in &bucket.entries {
                            if let Some(key) = child.key() {
                                f(key, child);
                            }
                        }
                    }
                }
                HeadTag::Branch4 => {
                    let node: *mut Branch4<KEY_LEN, O, S> = self.ptr();
                    for bucket in &(*node).child_table.buckets {
                        // TODO replace this with iterator
                        for child in &bucket.entries {
                            if let Some(key) = child.key() {
                                f(key, child);
                            }
                        }
                    }
                }
                HeadTag::Branch8 => {
                    let node: *mut Branch8<KEY_LEN, O, S> = self.ptr();
                    for bucket in &(*node).child_table.buckets {
                        // TODO replace this with iterator
                        for child in &bucket.entries {
                            if let Some(key) = child.key() {
                                f(key, child);
                            }
                        }
                    }
                }
                HeadTag::Branch16 => {
                    let node: *mut Branch16<KEY_LEN, O, S> = self.ptr();
                    for bucket in &(*node).child_table.buckets {
                        // TODO replace this with iterator
                        for child in &bucket.entries {
                            if let Some(key) = child.key() {
                                f(key, child);
                            }
                        }
                    }
                }
                HeadTag::Branch32 => {
                    let node: *mut Branch32<KEY_LEN, O, S> = self.ptr();
                    for bucket in &(*node).child_table.buckets {
                        // TODO replace this with iterator
                        for child in &bucket.entries {
                            if let Some(key) = child.key() {
                                f(key, child);
                            }
                        }
                    }
                }
                HeadTag::Branch64 => {
                    let node: *mut Branch64<KEY_LEN, O, S> = self.ptr();
                    for bucket in &(*node).child_table.buckets {
                        // TODO replace this with iterator
                        for child in &bucket.entries {
                            if let Some(key) = child.key() {
                                f(key, child);
                            }
                        }
                    }
                }
                HeadTag::Branch128 => {
                    let node: *mut Branch128<KEY_LEN, O, S> = self.ptr();
                    for bucket in &(*node).child_table.buckets {
                        // TODO replace this with iterator
                        for child in &bucket.entries {
                            if let Some(key) = child.key() {
                                f(key, child);
                            }
                        }
                    }
                }
                HeadTag::Branch256 => {
                    let node: *mut Branch256<KEY_LEN, O, S> = self.ptr();
                    for bucket in &(*node).child_table.buckets {
                        // TODO replace this with iterator
                        for child in &bucket.entries {
                            if let Some(key) = child.key() {
                                f(key, child);
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn infixes<const INFIX_LEN: usize, F>(
        &self,
        key: &[u8; KEY_LEN],
        at_depth: usize,
        start_depth: usize,
        end_depth: usize,
        f: F,
        out: &mut Vec<[u8; INFIX_LEN]>,
    ) where
        F: Copy + Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN],
    {
        unsafe {
            match self.tag() {
                HeadTag::Empty => return,
                HeadTag::Leaf => Leaf::<KEY_LEN>::infixes::<INFIX_LEN, O, S, F>(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    f,
                    out,
                ),
                HeadTag::Branch2 => Branch2::<KEY_LEN, O, S>::infixes(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    end_depth,
                    f,
                    out,
                ),
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::infixes(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    end_depth,
                    f,
                    out,
                ),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::infixes(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    end_depth,
                    f,
                    out,
                ),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::infixes(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    end_depth,
                    f,
                    out,
                ),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::infixes(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    end_depth,
                    f,
                    out,
                ),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::infixes(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    end_depth,
                    f,
                    out,
                ),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::infixes(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    end_depth,
                    f,
                    out,
                ),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::infixes(
                    self.ptr(),
                    key,
                    at_depth,
                    start_depth,
                    end_depth,
                    f,
                    out,
                ),
            }
        }
    }

    pub(crate) fn has_prefix(&self, at_depth: usize, key: &[u8; KEY_LEN], end_depth: usize) -> bool {
        unsafe {
            match self.tag() {
                HeadTag::Empty => end_depth < at_depth,
                HeadTag::Leaf => {
                    Leaf::<KEY_LEN>::has_prefix::<O>(self.ptr(), at_depth, key, end_depth)
                }
                HeadTag::Branch2 => {
                    Branch2::<KEY_LEN, O, S>::has_prefix(self.ptr(), at_depth, key, end_depth)
                }
                HeadTag::Branch4 => {
                    Branch4::<KEY_LEN, O, S>::has_prefix(self.ptr(), at_depth, key, end_depth)
                }
                HeadTag::Branch8 => {
                    Branch8::<KEY_LEN, O, S>::has_prefix(self.ptr(), at_depth, key, end_depth)
                }
                HeadTag::Branch16 => {
                    Branch16::<KEY_LEN, O, S>::has_prefix(self.ptr(), at_depth, key, end_depth)
                }
                HeadTag::Branch32 => {
                    Branch32::<KEY_LEN, O, S>::has_prefix(self.ptr(), at_depth, key, end_depth)
                }
                HeadTag::Branch64 => {
                    Branch64::<KEY_LEN, O, S>::has_prefix(self.ptr(), at_depth, key, end_depth)
                }
                HeadTag::Branch128 => {
                    Branch128::<KEY_LEN, O, S>::has_prefix(self.ptr(), at_depth, key, end_depth)
                }
                HeadTag::Branch256 => {
                    Branch256::<KEY_LEN, O, S>::has_prefix(self.ptr(), at_depth, key, end_depth)
                }
            }
        }
    }

    pub(crate) fn segmented_len(
        &self,
        at_depth: usize,
        key: &[u8; KEY_LEN],
        start_depth: usize,
    ) -> usize {
        unsafe {
            match self.tag() {
                HeadTag::Empty => 0,
                HeadTag::Leaf => {
                    Leaf::<KEY_LEN>::segmented_len::<O>(self.ptr(), at_depth, key, start_depth)
                }
                HeadTag::Branch2 => {
                    Branch2::<KEY_LEN, O, S>::segmented_len(self.ptr(), at_depth, key, start_depth)
                }
                HeadTag::Branch4 => {
                    Branch4::<KEY_LEN, O, S>::segmented_len(self.ptr(), at_depth, key, start_depth)
                }
                HeadTag::Branch8 => {
                    Branch8::<KEY_LEN, O, S>::segmented_len(self.ptr(), at_depth, key, start_depth)
                }
                HeadTag::Branch16 => {
                    Branch16::<KEY_LEN, O, S>::segmented_len(self.ptr(), at_depth, key, start_depth)
                }
                HeadTag::Branch32 => {
                    Branch32::<KEY_LEN, O, S>::segmented_len(self.ptr(), at_depth, key, start_depth)
                }
                HeadTag::Branch64 => {
                    Branch64::<KEY_LEN, O, S>::segmented_len(self.ptr(), at_depth, key, start_depth)
                }
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::segmented_len(
                    self.ptr(),
                    at_depth,
                    key,
                    start_depth,
                ),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::segmented_len(
                    self.ptr(),
                    at_depth,
                    key,
                    start_depth,
                ),
            }
        }
    }

    pub unsafe fn upsert<E, F>(&mut self, key: u8, update: E, insert: F)
    where
        E: Fn(&mut Head<KEY_LEN, O, S>),
        F: Fn(&mut Head<KEY_LEN, O, S>),
    {
        unsafe {
            match self.tag() {
                HeadTag::Empty => panic!("upsert on empty"),
                HeadTag::Leaf => panic!("upsert on leaf"),
                HeadTag::Branch2 => Branch2::<KEY_LEN, O, S>::upsert(self, key, update, insert),
                HeadTag::Branch4 => Branch4::<KEY_LEN, O, S>::upsert(self, key, update, insert),
                HeadTag::Branch8 => Branch8::<KEY_LEN, O, S>::upsert(self, key, update, insert),
                HeadTag::Branch16 => Branch16::<KEY_LEN, O, S>::upsert(self, key, update, insert),
                HeadTag::Branch32 => Branch32::<KEY_LEN, O, S>::upsert(self, key, update, insert),
                HeadTag::Branch64 => Branch64::<KEY_LEN, O, S>::upsert(self, key, update, insert),
                HeadTag::Branch128 => Branch128::<KEY_LEN, O, S>::upsert(self, key, update, insert),
                HeadTag::Branch256 => Branch256::<KEY_LEN, O, S>::upsert(self, key, update, insert),
            };
        }
    }

    pub(crate) fn union(&mut self, other: &Self, at_depth: usize) {
        if other.tag() == HeadTag::Empty {
            return;
        }

        if self.tag() == HeadTag::Empty {
            *self = other.clone();
            return;
        }

        let self_hash = self.hash();
        let other_hash = other.hash();
        if self_hash == other_hash {
            return;
        }
        let self_depth = self.end_depth();
        let other_depth = other.end_depth();
        unsafe {
            for depth in at_depth..std::cmp::min(self_depth, other_depth) {
                if self.peek(depth) != other.peek(depth) {
                    let new_branch = Branch2::new(depth);
                    Branch2::insert(new_branch, other.with_start(depth), other_hash);
                    Branch2::insert(new_branch, self.with_start(depth), self_hash);

                    *self = Head::new(HeadTag::Branch2, self.key().unwrap(), new_branch);
                    return;
                }
            }
            if self_depth < other_depth {
                self.upsert(
                    other.peek(self_depth),
                    |child| child.union(other, self_depth),
                    |head| head.insert(other.with_start(self_depth), other_hash),
                );
                return;
            }

            if other_depth < self_depth {
                let mut new_branch: Head<KEY_LEN, O, S> = other.with_start(at_depth);
                new_branch.upsert(
                    self.peek(other_depth),
                    |child| child.union(self, other_depth),
                    |head| head.insert(self.with_start(other_depth), self_hash),
                );
                debug_assert_eq!(self.key(), new_branch.key());
                *self = new_branch;
                return;
            }

            other.each_child(|other_key, other_child| {
                self.upsert(
                    other_key,
                    |child| child.union(other_child, self_depth),
                    |head| head.insert(other_child.clone(), other_child.hash()),
                );
            });
        }
    }
}

unsafe impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> ByteEntry
    for Head<KEY_LEN, O, S>
{
    fn key(&self) -> Option<u8> {
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
                HeadTag::Empty => Self::empty(),
                HeadTag::Leaf => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Leaf::<KEY_LEN>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch2 => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Branch2::<KEY_LEN, O, S>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch4 => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Branch4::<KEY_LEN, O, S>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch8 => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Branch8::<KEY_LEN, O, S>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch16 => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Branch16::<KEY_LEN, O, S>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch32 => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Branch32::<KEY_LEN, O, S>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch64 => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Branch64::<KEY_LEN, O, S>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch128 => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Branch128::<KEY_LEN, O, S>::rc_inc(self.ptr()),
                ),
                HeadTag::Branch256 => Self::new(
                    self.tag(),
                    self.key().unwrap(),
                    Branch256::<KEY_LEN, O, S>::rc_inc(self.ptr()),
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
                HeadTag::Empty => return,
                HeadTag::Leaf => Leaf::<KEY_LEN>::rc_dec(self.ptr()),
                HeadTag::Branch2 => Branch2::<KEY_LEN, O, S>::rc_dec(self.ptr()),
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
        Self::empty()
    }
}

#[derive(Debug, Clone)]
pub struct PATCH<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    root: Head<KEY_LEN, O, S>,
}

impl<const KEY_LEN: usize, O, S> PATCH<KEY_LEN, O, S>
where
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
{
    pub fn new() -> Self {
        PATCH {
            root: Head::<KEY_LEN, O, S>::empty(),
        }
    }

    pub fn put(&mut self, entry: &Entry<KEY_LEN>) {
        self.root.put(entry, 0);
    }

    pub fn len(&self) -> u64 {
        self.root.count()
    }

    pub fn infixes<const INFIX_LEN: usize, F>(
        &self,
        key: &[u8; KEY_LEN],
        start_depth: usize,
        end_depth: usize,
        f: F,
    ) -> Vec<[u8; INFIX_LEN]>
    where
        F: Copy + Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN],
    {
        let mut out = vec![];
        self.root.infixes(
            key,
            0,
            O::tree_index(start_depth),
            O::tree_index(end_depth),
            f,
            &mut out,
        );
        out
    }

    pub fn has_prefix(&self, key: &[u8; KEY_LEN], end_depth: usize) -> bool {
        self.root
            .has_prefix(0, key, O::tree_index(end_depth))
    }

    pub fn segmented_len(&self, key: &[u8; KEY_LEN], start_depth: usize) -> usize {
        self.root
            .segmented_len(0, key, O::tree_index(start_depth))
    }

    pub fn union(&mut self, other: &Self) {
        self.root.union(&other.root, 0);
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
        assert_eq!(
            unsafe {
                Head::<64, IdentityOrder, SingleSegmentation>::new::<u8>(
                    HeadTag::Empty,
                    0,
                    std::ptr::null_mut(),
                )
                .tag()
            },
            HeadTag::Empty
        );
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
            assert_eq!(head.key().unwrap(), k);
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
        init();

        let _tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
    }

    #[test]
    fn tree_put_one() {
        init();

        const KEY_SIZE: usize = 64;
        let mut tree = PATCH::<KEY_SIZE, IdentityOrder, SingleSegmentation>::new();
        let entry = Entry::new(&[0; KEY_SIZE]);
        tree.put(&entry);
    }

    #[test]
    fn branch_size() {
        assert_eq!(
            mem::size_of::<ByteTable2<Head<64, IdentityOrder, SingleSegmentation>>>(),
            16
        );
        assert_eq!(
            mem::size_of::<Branch2<64, IdentityOrder, SingleSegmentation>>(),
            64
        );
        assert_eq!(
            mem::size_of::<Branch4<64, IdentityOrder, SingleSegmentation>>(),
            48 + 16 * 2
        );
        assert_eq!(
            mem::size_of::<Branch8<64, IdentityOrder, SingleSegmentation>>(),
            48 + 16 * 4
        );
        assert_eq!(
            mem::size_of::<Branch16<64, IdentityOrder, SingleSegmentation>>(),
            48 + 16 * 8
        );
        assert_eq!(
            mem::size_of::<Branch32<64, IdentityOrder, SingleSegmentation>>(),
            48 + 16 * 16
        );
        assert_eq!(
            mem::size_of::<Branch64<64, IdentityOrder, SingleSegmentation>>(),
            48 + 16 * 32
        );
        assert_eq!(
            mem::size_of::<Branch128<64, IdentityOrder, SingleSegmentation>>(),
            48 + 16 * 64
        );
        assert_eq!(
            mem::size_of::<Branch256<64, IdentityOrder, SingleSegmentation>>(),
            48 + 16 * 128
        );
    }

    proptest! {
    #[test]
    fn tree_put(keys in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
        let mut tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        for key in keys {
            let key: [u8; 64] = key.try_into().unwrap();
            let entry = Entry::new(&key);
            tree.put(&entry);
        }
    }

    #[test]
    fn tree_len(keys in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
        let mut tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        let mut set = HashSet::new();
        for key in keys {
            let key: [u8; 64] = key.try_into().unwrap();
            let entry = Entry::new(&key);
            tree.put(&entry);
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
            tree.put(&entry);
            set.insert(key);
        }
        let mut set_vec = Vec::from_iter(set.into_iter());
        let mut tree_vec = tree.infixes(&[0; 64], 0, 63, |x| x);

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
            left_tree.put(&entry);
            set.insert(key);
        }

        let mut right_tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
        for entry in right {
            let mut key = [0; 64];
            key.iter_mut().set_from(entry.iter().cloned());
            let entry = Entry::new(&key);
            right_tree.put(&entry);
            set.insert(key);
        }

        left_tree.union(&right_tree);

        let mut set_vec = Vec::from_iter(set.into_iter());
        let mut tree_vec = left_tree.infixes(&[0; 64], 0, 63, |x| x);

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
            left_tree.put(&entry);
            set.insert(key);
        }

        let right_tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();

        left_tree.union(&right_tree);

        let mut set_vec = Vec::from_iter(set.into_iter());
        let mut tree_vec = left_tree.infixes(&[0; 64], 0, 63, |x| x);

        set_vec.sort();
        tree_vec.sort();

        prop_assert_eq!(set_vec, tree_vec);
        }
    }
}
