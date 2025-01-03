use super::*;
use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::alloc::{alloc, dealloc, Layout};

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Branch<
    const KEY_LEN: usize,
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
    Table: ?Sized,
> {
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<S>,

    rc: atomic::AtomicU32,
    pub end_depth: u32,
    pub childleaf: *const Leaf<KEY_LEN>,
    pub leaf_count: u64,
    pub segment_count: u64,
    pub hash: u128,
    pub child_table: Table,
}

pub(crate) type Branch2<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body for Branch2<KEY_LEN, O, S> {
    const TAG: HeadTag = HeadTag::Branch2;
}

pub(crate) type Branch4<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body for Branch4<KEY_LEN, O, S> {
    const TAG: HeadTag = HeadTag::Branch4;
}

pub(crate) type Branch8<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body for Branch8<KEY_LEN, O, S> {
    const TAG: HeadTag = HeadTag::Branch8;
}

pub(crate) type Branch16<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body for Branch16<KEY_LEN, O, S> {
    const TAG: HeadTag = HeadTag::Branch16;
}

pub(crate) type Branch32<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body for Branch32<KEY_LEN, O, S> {
    const TAG: HeadTag = HeadTag::Branch32;
}

pub(crate) type Branch64<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body for Branch64<KEY_LEN, O, S> {
    const TAG: HeadTag = HeadTag::Branch64;
}

pub(crate) type Branch128<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body for Branch128<KEY_LEN, O, S> {
    const TAG: HeadTag = HeadTag::Branch128;
}

pub(crate) type Branch256<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 256]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body for Branch256<KEY_LEN, O, S> {
    const TAG: HeadTag = HeadTag::Branch256;
}

pub(crate) type BranchN<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    BranchN<KEY_LEN, O, S>
{
    pub fn count_segment(&self, at_depth: usize) -> u64 {
        if S::segment(O::key_index(at_depth))
            != S::segment(O::key_index(self.end_depth as usize))
        {
            1
        } else {
            self.segment_count
        }
    }

    pub fn childleaf_key(&self) -> &[u8; KEY_LEN] {
        unsafe { &(*self.childleaf).key }
    }

    pub(super) fn infixes<const PREFIX_LEN: usize, const INFIX_LEN: usize, F>(
        &self,
        prefix: &[u8; PREFIX_LEN],
        at_depth: usize,
        f: &mut F,
    ) where
        F: FnMut(&[u8; INFIX_LEN]),
    {
        let node_end_depth = self.end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = self.childleaf_key();
        for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
            if leaf_key[O::key_index(depth)] != prefix[depth] {
                return;
            }
        }

        // The infix ends within the current node.
        if PREFIX_LEN + INFIX_LEN <= node_end_depth {
            let infix: [u8; INFIX_LEN] =
                core::array::from_fn(|i| self.childleaf_key()[O::key_index(PREFIX_LEN + i)]);
            f(&infix);
            return;
        }
        // The prefix ends in a child of this node.
        if PREFIX_LEN > node_end_depth {
            if let Some(child) = self.child_table.table_get(prefix[node_end_depth]) {
                child.infixes(prefix, node_end_depth, f);
            }
            return;
        }

        // The prefix ends in this node, but the infix ends in a child.
        for entry in &self.child_table {
            if let Some(entry) = entry {
                entry.infixes(prefix, node_end_depth, f);
            }
        }
    }

    pub(super) fn has_prefix<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        let node_end_depth = self.end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = self.childleaf_key();
        for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
            if leaf_key[O::key_index(depth)] != prefix[depth] {
                return false;
            }
        }

        // The prefix ends in this node.
        if PREFIX_LEN <= node_end_depth {
            return true;
        }

        //The prefix ends in a child of this node.
        if let Some(child) = self.child_table.table_get(prefix[node_end_depth]) {
            return child.has_prefix(node_end_depth, prefix);
        }
        // This node doesn't have a child matching the prefix.
        return false;
    }

    pub(super) fn segmented_len<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        let node_end_depth = self.end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = self.childleaf_key();
        for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
            let key_depth = O::key_index(depth);
            if leaf_key[key_depth] != prefix[depth] {
                return 0;
            }
        }
        if PREFIX_LEN <= node_end_depth {
            if S::segment(O::key_index(PREFIX_LEN)) != S::segment(O::key_index(node_end_depth)) {
                return 1;
            } else {
                return self.segment_count;
            }
        }
        if let Some(child) = self.child_table.table_get(prefix[node_end_depth]) {
            return child.segmented_len(node_end_depth, prefix);
        }
        return 0;
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 2]>
{
    pub(super) fn new(
        head_key: u8,
        end_depth: usize,
        child: Head<KEY_LEN, O, S>,
    ) -> Head<KEY_LEN, O, S> {
        unsafe {
            let layout = Layout::new::<Self>();
            if let Some(ptr) = NonNull::new(alloc(layout) as *mut Self) {
                ptr.write(
                    Self {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf: child.childleaf(),
                        leaf_count: child.count(),
                        segment_count: child.count_segment(end_depth),
                        hash: child.hash(),
                        child_table: [Some(child), None],
                    },
                );
    
                Head::new(head_key, ptr)
            } else {
                panic!("Allocation failed!");
            }
        }
    }
}

impl<
        const KEY_LEN: usize,
        const SLOT_COUNT: usize,
        O: KeyOrdering<KEY_LEN>,
        S: KeySegmentation<KEY_LEN>,
    > Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; SLOT_COUNT]>
{
    pub(super) unsafe fn rc_inc(branch: NonNull<Self>) -> NonNull<Self> {
        unsafe {
            let branch = branch.as_ptr();
            let mut current = (*branch).rc.load(Relaxed);
            loop {
                if current == u32::MAX {
                    panic!("max refcount exceeded");
                }
                match (*branch)
                    .rc
                    .compare_exchange(current, current + 1, Relaxed, Relaxed)
                {
                    Ok(_) => return NonNull::new_unchecked(branch),
                    Err(v) => current = v,
                }
            }
        }
    }

    pub(super) unsafe fn rc_dec(branch: NonNull<Self>) {
        unsafe {
            let branch = branch.as_ptr();
            if (*branch).rc.fetch_sub(1, Release) != 1 {
                return;
            }
            (*branch).rc.load(Acquire);

            std::ptr::drop_in_place(branch);

            let layout = Layout::new::<Self>();
            let ptr = branch as *mut u8;
            dealloc(ptr, layout);
        }
    }

    pub(super) unsafe fn rc_cow(branch: NonNull<Self>) -> Option<NonNull<Self>> {
        unsafe {
            let branch = branch.as_ptr();
            if (*branch).rc.load(Acquire) == 1 {
                None
            } else {
                let layout = Layout::new::<Self>();
                if let Some(ptr) = NonNull::new(alloc(layout) as *mut Self) {
                    ptr.write(Self {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: (*branch).end_depth,
                        childleaf: (*branch).childleaf,
                        leaf_count: (*branch).leaf_count,
                        segment_count: (*branch).segment_count,
                        hash: (*branch).hash,
                        child_table: (*branch).child_table.clone(),
                    });
                    Self::rc_dec(NonNull::new_unchecked(branch));
                    Some(ptr)
                } else {
                    panic!("Allocation failed!");
                }
            }
        }
    }
}

macro_rules! create_grow {
    ($base_size:expr, $grown_size:expr) => {
        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $base_size]>
        {
            pub(super) fn grow(
                branch: NonNull<Self>,
            ) -> NonNull<Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $grown_size]>> {
                unsafe {
                    let branch = branch.as_ptr();
                    let layout = Layout::new::<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $grown_size]>,
                    >();
                    if let Some(ptr) = NonNull::new(alloc(layout)
                        as *mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $grown_size]>)
                    {
                        ptr.write(
                            Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $grown_size]> {
                                key_ordering: PhantomData,
                                key_segments: PhantomData,
                                rc: atomic::AtomicU32::new(1),
                                end_depth: (*branch).end_depth,
                                leaf_count: (*branch).leaf_count,
                                segment_count: (*branch).segment_count,
                                childleaf: (*branch).childleaf,
                                hash: (*branch).hash,
                                child_table: std::array::from_fn(|_| None),
                            },
                        );

                        (*branch).child_table.table_grow(&mut (*ptr.as_ptr()).child_table);

                        Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $base_size]>::rc_dec(
                            NonNull::new_unchecked(branch),
                        );

                        ptr
                    } else {
                        panic!("Allocation failed!");
                    }
                }
            }
        }
    };
}

create_grow!(2, 4);
create_grow!(4, 8);
create_grow!(8, 16);
create_grow!(16, 32);
create_grow!(32, 64);
create_grow!(64, 128);
create_grow!(128, 256);
