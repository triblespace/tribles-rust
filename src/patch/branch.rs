use super::*;
use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::alloc::{alloc, dealloc, Layout};
use std::convert::TryInto;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(super) struct Metadata {
    pub leaf_count: u64,
    pub segment_count: u64,
    pub hash: u128,
}

macro_rules! create_branch {
    ($name:ident, $table:tt) => {
        #[derive(Debug)]
        #[repr(C)]
        pub(super) struct $name<
            const KEY_LEN: usize,
            O: KeyOrdering<KEY_LEN>,
            S: KeySegmentation<KEY_LEN>,
        > {
            key_ordering: PhantomData<O>,
            key_segments: PhantomData<S>,

            rc: atomic::AtomicU32,
            pub end_depth: u32,
            pub childleaf: *const Leaf<KEY_LEN>,
            pub metadata: Metadata,
            pub child_table: $table<Head<KEY_LEN, O, S>>,
        }

        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            $name<KEY_LEN, O, S>
        {
            pub(super) unsafe fn rc_inc(head: &Head<KEY_LEN, O, S>) -> Head<KEY_LEN, O, S> {
                debug_assert!(head.tag() == HeadTag::$name);
                unsafe {
                    let node: *mut Self = head.ptr();
                    let mut current = (*node).rc.load(Relaxed);
                    loop {
                        if current == u32::MAX {
                            panic!("max refcount exceeded");
                        }
                        match (*node)
                            .rc
                            .compare_exchange(current, current + 1, Relaxed, Relaxed)
                        {
                            Ok(_) => {
                                return Head::<KEY_LEN, O, S>::new(
                                    HeadTag::$name,
                                    head.key().unwrap(),
                                    node,
                                )
                            }
                            Err(v) => current = v,
                        }
                    }
                }
            }

            pub(super) unsafe fn rc_dec(head: &mut Head<KEY_LEN, O, S>) {
                debug_assert!(head.tag() == HeadTag::$name);
                unsafe {
                    let node: *mut Self = head.ptr();
                    if (*node).rc.fetch_sub(1, Release) != 1 {
                        return;
                    }
                    (*node).rc.load(Acquire);

                    std::ptr::drop_in_place(node);

                    let layout = Layout::new::<Self>();
                    let ptr = node as *mut u8;
                    dealloc(ptr, layout);
                }
            }

            pub(super) unsafe fn rc_cow(head: &mut Head<KEY_LEN, O, S>) {
                debug_assert!(head.tag() == HeadTag::$name);
                unsafe {
                    let node: *const Self = head.ptr();
                    if (*node).rc.load(Acquire) != 1 {
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
                                end_depth: (*node).end_depth,
                                childleaf: (*node).childleaf,
                                metadata: (*node).metadata,
                                child_table: (*node).child_table.clone(),
                            },
                        );

                        *head = Head::new(HeadTag::$name, head.key().unwrap(), ptr);
                    }
                }
            }

            pub unsafe fn count_segment(node: *const Self, at_depth: usize) -> u64 {
                if S::segment(O::key_index(at_depth))
                    != S::segment(O::key_index((*node).end_depth as usize))
                {
                    1
                } else {
                    (*node).metadata.segment_count
                }
            }

            pub unsafe fn each_child<F>(head: Head<KEY_LEN, O, S>, mut f: F)
            where
                F: FnMut(Head<KEY_LEN, O, S>),
            {
                debug_assert!(head.tag() == HeadTag::$name);
                unsafe {
                    let node: *mut Self = head.ptr();
                    if (*node).rc.load(Acquire) == 1 {
                        for child in &mut (*node).child_table {
                            if let Some(_) = child.key() {
                                f(std::mem::replace(child, Head::empty()));
                            }
                        }
                    } else {
                        for child in &(*node).child_table {
                            if let Some(_) = child.key() {
                                f(child.clone());
                            }
                        }
                    }
                }
            }

            pub(super) unsafe fn infixes<const PREFIX_LEN: usize, const INFIX_LEN: usize, F>(
                head: &Head<KEY_LEN, O, S>,
                prefix: &[u8; PREFIX_LEN],
                at_depth: usize,
                f: &mut F,
            ) where
                F: FnMut([u8; INFIX_LEN]),
            {
                debug_assert!(head.tag() == HeadTag::$name);
                let node: *const Self = head.ptr();
                let node_end_depth = ((*node).end_depth as usize);
                let leaf_key: &[u8; KEY_LEN] = &(*(*node).childleaf).key;
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
                    let infix = (*(*node).childleaf).key
                        [O::key_index(PREFIX_LEN)..=O::key_index(PREFIX_LEN + INFIX_LEN - 1)]
                        .try_into()
                        .expect("invalid infix range");
                    f(infix);
                    return;
                }
                // The prefix ends in a child of this node.
                if PREFIX_LEN > node_end_depth {
                    if let Some(child) = (*node).child_table.get(prefix[node_end_depth]) {
                        child.infixes(prefix, node_end_depth, f);
                    }
                    return;
                }

                // The prefix ends in this node, but the infix ends in a child.
                for entry in &(*node).child_table {
                    entry.infixes(prefix, node_end_depth, f);
                }
            }

            pub(super) unsafe fn has_prefix<const PREFIX_LEN: usize>(
                node: *const Self,
                at_depth: usize,
                prefix: &[u8; PREFIX_LEN],
            ) -> bool {
                let node_end_depth = ((*node).end_depth as usize);
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
                if let Some(child) = (*node).child_table.get(prefix[node_end_depth]) {
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
                let node_end_depth = ((*node).end_depth as usize);
                let leaf_key: &[u8; KEY_LEN] = &(*(*node).childleaf).key;
                for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
                    let key_depth = O::key_index(depth);
                    if leaf_key[key_depth] != prefix[depth] {
                        return 0;
                    }
                }
                if PREFIX_LEN <= node_end_depth {
                    if S::segment(O::key_index(PREFIX_LEN))
                        != S::segment(O::key_index(node_end_depth))
                    {
                        return 1;
                    } else {
                        return (*node).metadata.segment_count;
                    }
                }
                if let Some(child) = (*node).child_table.get(prefix[node_end_depth]) {
                    return child.segmented_len(node_end_depth, prefix);
                }
                return 0;
            }
        }
    };
}

create_branch!(Branch2, ByteTable2);
create_branch!(Branch4, ByteTable4);
create_branch!(Branch8, ByteTable8);
create_branch!(Branch16, ByteTable16);
create_branch!(Branch32, ByteTable32);
create_branch!(Branch64, ByteTable64);
create_branch!(Branch128, ByteTable128);
create_branch!(Branch256, ByteTable256);

macro_rules! create_grow {
    () => {};
    ($name:ident, $grown_name:ident) => {
        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            $name<KEY_LEN, O, S>
        {
            pub(super) fn grow(head: &mut Head<KEY_LEN, O, S>) {
                debug_assert!(head.tag() == HeadTag::$name);
                unsafe {
                    let node: *const Self = head.ptr();
                    let layout = Layout::new::<$grown_name<KEY_LEN, O, S>>();
                    let ptr = alloc(layout) as *mut $grown_name<KEY_LEN, O, S>;
                    if ptr.is_null() {
                        panic!("Allocation failed!");
                    }
                    std::ptr::write(
                        ptr,
                        $grown_name::<KEY_LEN, O, S> {
                            key_ordering: PhantomData,
                            key_segments: PhantomData,
                            rc: atomic::AtomicU32::new(1),
                            end_depth: (*node).end_depth,
                            childleaf: (*node).childleaf,
                            metadata: (*node).metadata,
                            child_table: (*node).child_table.grow(),
                        },
                    );

                    *head = Head::new(HeadTag::$grown_name, head.key().unwrap(), ptr);
                }
            }
        }
    };
}

create_grow!(Branch2, Branch4);
create_grow!(Branch4, Branch8);
create_grow!(Branch8, Branch16);
create_grow!(Branch16, Branch32);
create_grow!(Branch32, Branch64);
create_grow!(Branch64, Branch128);
create_grow!(Branch128, Branch256);

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Branch2<KEY_LEN, O, S>
{
    pub(super) fn new(
        head_key: u8,
        end_depth: usize,
        child_a: Head<KEY_LEN, O, S>,
        child_hash_a: u128,
        child_b: Head<KEY_LEN, O, S>,
        child_hash_b: u128,
    ) -> Head<KEY_LEN, O, S> {
        unsafe {
            debug_assert!(child_a.key() != None);
            debug_assert!(child_b.key() != None);
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
                    childleaf: child_a.childleaf(),
                    metadata: Metadata {
                        leaf_count: child_a.count() + child_b.count(),
                        segment_count: child_a.count_segment(end_depth)
                            + child_b.count_segment(end_depth),
                        hash: child_hash_a ^ child_hash_b,
                    },
                    child_table: ByteTable2::new_with(child_a, child_b),
                },
            );

            Head::new(HeadTag::Branch2, head_key, ptr)
        }
    }
}
