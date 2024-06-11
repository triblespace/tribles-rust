use super::*;
use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::alloc::{alloc, dealloc, Layout};
use std::convert::TryInto;

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
pub(crate) type Branch4<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 4]>;
pub(crate) type Branch8<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 8]>;
pub(crate) type Branch16<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 16]>;
pub(crate) type Branch32<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 32]>;
pub(crate) type Branch64<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 64]>;
pub(crate) type Branch128<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 128]>;
pub(crate) type Branch256<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; 256]>;
pub(crate) type BranchN<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    BranchN<KEY_LEN, O, S>
{
    pub fn count_segment(branch: *const Self, at_depth: usize) -> u64 {
        unsafe {
            if S::segment(O::key_index(at_depth))
                != S::segment(O::key_index((*branch).end_depth as usize))
            {
                1
            } else {
                (*branch).segment_count
            }
        }
    }

    pub unsafe fn take_or_clone_children<F>(branch: *mut Self, mut f: F)
    where
        F: FnMut(Head<KEY_LEN, O, S>),
    {
        if (*branch).rc.load(Acquire) == 1 {
            for child in &mut (*branch).child_table {
                if let Some(child) = child.take() {
                    f(child);
                }
            }
        } else {
            for child in &(*branch).child_table {
                if let Some(child) = child {
                    f(child.clone());
                }
            }
        }
    }

    pub(super) unsafe fn infixes<const PREFIX_LEN: usize, const INFIX_LEN: usize, F>(
        branch: *mut Self,
        prefix: &[u8; PREFIX_LEN],
        at_depth: usize,
        f: &mut F,
    ) where
        F: FnMut([u8; INFIX_LEN]),
    {
        let node_end_depth = (*branch).end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = &(*(*branch).childleaf).key;
        for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
            if leaf_key[O::key_index(depth)] != prefix[depth] {
                return;
            }
        }

        // The infix ends within the current node.
        if PREFIX_LEN + INFIX_LEN <= node_end_depth {
            // It has to be `..=O::key_index(PREFIX_LEN + INFIX_LEN - 1)`
            // because `..O::key_index(PREFIX_LEN + INFIX_LEN)` would not work,
            // since `key_index` is not monotonic,
            // so the next segment might be somewhere else.
            let infix = (*(*branch).childleaf).key
                [O::key_index(PREFIX_LEN)..=O::key_index(PREFIX_LEN + INFIX_LEN - 1)]
                .try_into()
                .expect("invalid infix range");
            f(infix);
            return;
        }
        // The prefix ends in a child of this node.
        if PREFIX_LEN > node_end_depth {
            if let Some(child) = (*branch).child_table.table_get(prefix[node_end_depth]) {
                child.infixes(prefix, node_end_depth, f);
            }
            return;
        }

        // The prefix ends in this node, but the infix ends in a child.
        for entry in &(*branch).child_table {
            if let Some(entry) = entry {
                entry.infixes(prefix, node_end_depth, f);
            }
        }
    }

    pub(super) unsafe fn has_prefix<const PREFIX_LEN: usize>(
        node: *const Self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        let node_end_depth = (*node).end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = &(*(*node).childleaf).key;
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
        if let Some(child) = (*node).child_table.table_get(prefix[node_end_depth]) {
            return child.has_prefix(node_end_depth, prefix);
        }
        // This node doesn't have a child matching the prefix.
        return false;
    }

    pub(super) unsafe fn segmented_len<const PREFIX_LEN: usize>(
        node: *const Self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        let node_end_depth = (*node).end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = &(*(*node).childleaf).key;
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
                return (*node).segment_count;
            }
        }
        if let Some(child) = (*node).child_table.table_get(prefix[node_end_depth]) {
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
            let ptr = alloc(layout) as *mut Self;
            if ptr.is_null() {
                panic!("Allocation failed!");
            }
            std::ptr::write(
                ptr,
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

            Head::new(HeadTag::Branch2, head_key, ptr)
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
    pub(super) unsafe fn rc_inc(branch: *mut Self) -> *mut Self {
        unsafe {
            let mut current = (*branch).rc.load(Relaxed);
            loop {
                if current == u32::MAX {
                    panic!("max refcount exceeded");
                }
                match (*branch)
                    .rc
                    .compare_exchange(current, current + 1, Relaxed, Relaxed)
                {
                    Ok(_) => return branch,
                    Err(v) => current = v,
                }
            }
        }
    }

    pub(super) unsafe fn rc_dec(branch: *mut Self) {
        unsafe {
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

    pub(super) unsafe fn rc_cow(branch: *const Self) -> Option<*mut Self> {
        unsafe {
            if (*branch).rc.load(Acquire) == 1 {
                None
            } else {
                let layout = Layout::new::<Self>();
                let ptr = alloc(layout) as *mut Self;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Self {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: (*branch).end_depth,
                        childleaf: (*branch).childleaf,
                        leaf_count: (*branch).leaf_count,
                        segment_count: (*branch).segment_count,
                        hash: (*branch).hash,
                        child_table: (*branch).child_table.clone(),
                    },
                );
                Some(ptr)
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
                branch: *mut Self,
            ) -> *mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $grown_size]> {
                unsafe {
                    let layout = Layout::new::<
                        Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $grown_size]>,
                    >();
                    let ptr = alloc(layout)
                        as *mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; $grown_size]>;
                    if ptr.is_null() {
                        panic!("Allocation failed!");
                    }
                    std::ptr::write(
                        ptr,
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

                    (*branch).child_table.table_grow(&mut (*ptr).child_table);

                    ptr
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

pub(super) fn branch_from<
    const KEY_LEN: usize,
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
>(
    end_depth: usize,
    children: Vec<Head<KEY_LEN, O, S>>,
) -> Head<KEY_LEN, O, S> {
    unsafe {
        let childleaf = children[0].childleaf();
        let leaf_count = children.iter().map(|c| c.count()).sum();
        let segment_count = children.iter().map(|c| c.count_segment(end_depth)).sum();
        let hash = children
            .iter()
            .map(|c| c.hash())
            .reduce(|x, y| x ^ y)
            .unwrap();

        match children.len() {
            1..=2 => {
                let layout = Layout::new::<Branch2<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut Branch2<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Branch2 {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf,
                        leaf_count,
                        segment_count,
                        hash,
                        child_table: std::array::from_fn(|_| None),
                    },
                );

                let mut branch = Head::new(HeadTag::Branch2, 0, ptr);
                for child in children {
                    branch.insert_child(child);
                }
                branch
            }
            3..=4 => unsafe {
                let layout = Layout::new::<Branch4<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut Branch4<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Branch4 {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf,
                        leaf_count,
                        segment_count,
                        hash,
                        child_table: std::array::from_fn(|_| None),
                    },
                );

                let mut branch = Head::new(HeadTag::Branch4, 0, ptr);
                for child in children {
                    branch.insert_child(child);
                }
                branch
            },
            5..=8 => {
                let layout = Layout::new::<Branch8<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut Branch8<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Branch8 {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf,
                        leaf_count,
                        segment_count,
                        hash,
                        child_table: std::array::from_fn(|_| None),
                    },
                );

                let mut branch = Head::new(HeadTag::Branch8, 0, ptr);
                for child in children {
                    branch.insert_child(child);
                }
                branch
            }
            9..=16 => {
                let layout = Layout::new::<Branch16<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut Branch16<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Branch16 {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf,
                        leaf_count,
                        segment_count,
                        hash,
                        child_table: std::array::from_fn(|_| None),
                    },
                );

                let mut branch = Head::new(HeadTag::Branch16, 0, ptr);
                for child in children {
                    branch.insert_child(child);
                }
                branch
            }
            17..=32 => {
                let layout = Layout::new::<Branch32<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut Branch32<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Branch32 {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf,
                        leaf_count,
                        segment_count,
                        hash,
                        child_table: std::array::from_fn(|_| None),
                    },
                );

                let mut branch = Head::new(HeadTag::Branch32, 0, ptr);
                for child in children {
                    branch.insert_child(child);
                }
                branch
            }
            33..=64 => {
                let layout = Layout::new::<Branch64<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut Branch64<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Branch64 {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf,
                        leaf_count,
                        segment_count,
                        hash,
                        child_table: std::array::from_fn(|_| None),
                    },
                );

                let mut branch = Head::new(HeadTag::Branch64, 0, ptr);
                for child in children {
                    branch.insert_child(child);
                }
                branch
            }
            65..=128 => {
                let layout = Layout::new::<Branch128<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut Branch128<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Branch128 {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf,
                        leaf_count,
                        segment_count,
                        hash,
                        child_table: std::array::from_fn(|_| None),
                    },
                );

                let mut branch = Head::new(HeadTag::Branch128, 0, ptr);
                for child in children {
                    branch.insert_child(child);
                }
                branch
            }
            129..=256 => {
                let layout = Layout::new::<Branch256<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut Branch256<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                std::ptr::write(
                    ptr,
                    Branch256 {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        childleaf,
                        leaf_count,
                        segment_count,
                        hash,
                        child_table: std::array::from_fn(|_| None),
                    },
                );

                let mut branch = Head::new(HeadTag::Branch256, 0, ptr);
                for child in children {
                    branch.insert_child(child);
                }
                branch
            }
            _ => panic!("bad child count for branch"),
        }
    }
}
