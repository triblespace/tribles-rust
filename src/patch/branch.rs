use super::*;
use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::alloc::{alloc, dealloc, Layout};

fn min_key<const KEY_LEN: usize, V: Clone>(
    l: *const Leaf<KEY_LEN, V>,
    r: *const Leaf<KEY_LEN, V>,
) -> *const Leaf<KEY_LEN, V> {
    if l.is_null() {
        return r;
    }
    return l;
}

macro_rules! create_branch {
    ($name:ident, $table:tt) => {
        #[derive(Debug)]
        #[repr(C)]
        pub(super) struct $name<
            const KEY_LEN: usize,
            O: KeyOrdering<KEY_LEN>,
            S: KeySegmentation<KEY_LEN>,
            V: Clone
        > {
            key_ordering: PhantomData<O>,
            key_segments: PhantomData<S>,

            rc: atomic::AtomicU32,
            pub end_depth: u32,
            pub min: *const Leaf<KEY_LEN, V>,
            pub leaf_count: u64,
            pub segment_count: u64,
            pub hash: u128,
            pub child_table: $table<Head<KEY_LEN, O, S, V>>,
        }

        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>, V: Clone>
            $name<KEY_LEN, O, S, V>
        {
            pub(super) fn new(end_depth: usize) -> *mut Self {
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
                            min: std::ptr::null_mut(),
                            leaf_count: 0,
                            segment_count: 0,
                            hash: 0,
                            child_table: $table::new(),
                        },
                    );

                    ptr
                }
            }

            pub(super) unsafe fn rc_inc(node: *mut Self) -> *mut Self {
                unsafe {
                    let mut current = (*node).rc.load(Relaxed);
                    loop {
                        if current == u32::MAX {
                            panic!("max refcount exceeded");
                        }
                        match (*node)
                            .rc
                            .compare_exchange(current, current + 1, Relaxed, Relaxed)
                        {
                            Ok(_) => return node,
                            Err(v) => current = v,
                        }
                    }
                }
            }

            pub(super) unsafe fn rc_dec(node: *mut Self) {
                unsafe {
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

            pub(super) unsafe fn rc_mut(head: &mut Head<KEY_LEN, O, S, V>) -> *mut Self {
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
                                min: (*node).min,
                                leaf_count: (*node).leaf_count,
                                segment_count: (*node).segment_count,
                                hash: (*node).hash,
                                child_table: (*node).child_table.clone(),
                            },
                        );

                        *head = Head::new(HeadTag::$name, head.key().unwrap(), ptr);
                    }
                    head.ptr()
                }
            }

            pub unsafe fn count_segment(node: *const Self, at_depth: usize) -> u64 {
                if S::segment(O::key_index(at_depth))
                    != S::segment(O::key_index((*node).end_depth as usize))
                {
                    1
                } else {
                    (*node).segment_count
                }
            }

            pub unsafe fn insert_child(
                node: *mut Self,
                child: Head<KEY_LEN, O, S, V>,
                child_hash: u128,
            ) -> Head<KEY_LEN, O, S, V> {
                if let Some(_key) = child.key() {
                    let end_depth = (*node).end_depth as usize;
                    (*node).min = min_key((*node).min, child.min());
                    (*node).leaf_count += child.count();
                    (*node).segment_count += child.count_segment(end_depth);
                    (*node).hash ^= child_hash;
                    (*node).child_table.insert(child)
                } else {
                    Head::empty()
                }
            }

            pub unsafe fn upsert<E, F>(
                head: &mut Head<KEY_LEN, O, S, V>,
                key: u8,
                update: E,
                insert: F,
            ) where
                E: Fn(&mut Head<KEY_LEN, O, S, V>),
                F: Fn(&mut Head<KEY_LEN, O, S, V>),
            {
                debug_assert!(head.tag() == HeadTag::$name);
                let inner = Self::rc_mut(head);
                if let Some(child) = (*inner).child_table.get_mut(key) {
                    let old_child_hash = child.hash();
                    let old_child_segment_count = child.count_segment((*inner).end_depth as usize);
                    let old_child_leaf_count = child.count();

                    update(child);

                    (*inner).hash = ((*inner).hash ^ old_child_hash) ^ child.hash();
                    (*inner).segment_count = ((*inner).segment_count - old_child_segment_count)
                        + child.count_segment((*inner).end_depth as usize);
                    (*inner).leaf_count =
                        ((*inner).leaf_count - old_child_leaf_count) + child.count();
                } else {
                    insert(head);
                }
            }

            pub unsafe fn peek(node: *const Self, at_depth: usize) -> u8 {
                Leaf::<KEY_LEN, V>::peek((*node).min, at_depth)
            }

            pub unsafe fn insert(
                head: &mut Head<KEY_LEN, O, S, V>,
                entry: &Entry<KEY_LEN, V>,
                start_depth: usize,
            ) {
                debug_assert!(head.tag() == HeadTag::$name);
                let node: *const Self = head.ptr();
                let end_depth = (*node).end_depth as usize;

                let leaf_key: &[u8; KEY_LEN] = &(*(*node).min).key;
                for depth in start_depth..end_depth {
                    let key_depth = O::key_index(depth);
                    if leaf_key[key_depth] != entry.peek(key_depth) {
                        let new_branch = Branch2::new(depth);
                        Branch2::insert_child(new_branch, entry.leaf(depth), entry.hash);
                        Branch2::insert_child(new_branch, head.with_start(depth), head.hash());

                        *head = Head::new(HeadTag::Branch2, head.key().unwrap(), new_branch);
                        return;
                    }
                }

                let inner = Self::rc_mut(head);
                let key_end_depth = O::key_index(end_depth);
                if let Some(child) = (*inner).child_table.get_mut(entry.peek(key_end_depth)) {
                    let old_child_hash = child.hash();
                    let old_child_segment_count = child.count_segment(end_depth);
                    let old_child_leaf_count = child.count();

                    child.insert(entry, end_depth);

                    (*inner).hash = ((*inner).hash ^ old_child_hash) ^ child.hash();
                    (*inner).segment_count = ((*inner).segment_count - old_child_segment_count)
                        + child.count_segment(end_depth);
                    (*inner).leaf_count =
                        ((*inner).leaf_count - old_child_leaf_count) + child.count();

                    return;
                } else {
                    let displaced = Self::insert_child(inner, entry.leaf(end_depth), entry.hash);
                    if None != displaced.key() {
                        head.growing_reinsert(displaced);
                    }
                    return;
                }
            }

            pub(super) fn get(
                node: *const Self,
                at_depth: usize,
                key: &[u8; KEY_LEN],
            ) -> Option<V> {
                let node_end_depth = (unsafe{(*node).end_depth} as usize);
                let leaf_key: &[u8; KEY_LEN] = unsafe{&(*(*node).min).key};
                for depth in at_depth..node_end_depth {
                    let key_depth = O::key_index(depth);
                    if leaf_key[key_depth] != key[key_depth] {
                        return None;
                    }
                }
                if let Some(child) = unsafe{(*node).child_table.get(key[O::key_index(node_end_depth)])} {
                    return child.get(node_end_depth, key);
                }
                return None;
            }

            pub(super) unsafe fn infixes<const INFIX_LEN: usize, F>(
                node: *const Self,
                key: &[u8; KEY_LEN],
                at_depth: usize,
                start_depth: usize,
                end_depth: usize,
                f: F,
                out: &mut Vec<[u8; INFIX_LEN]>,
            ) where
                F: Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN] + Copy,
            {
                let node_end_depth = ((*node).end_depth as usize);
                let leaf_key: &[u8; KEY_LEN] = &(*(*node).min).key;
                for depth in at_depth..std::cmp::min(node_end_depth, start_depth) {
                    let key_depth = O::key_index(depth);
                    if leaf_key[key_depth] != key[key_depth] {
                        return;
                    }
                }

                if end_depth < node_end_depth {
                    out.push(f((*(*node).min).key));
                    return;
                }
                if start_depth > node_end_depth {
                    if let Some(child) = (*node).child_table.get(key[O::key_index(node_end_depth)]) {
                        child.infixes(key, node_end_depth, start_depth, end_depth, f, out);
                    }
                    return;
                }
                for bucket in &(*node).child_table.buckets {
                    // TODO replace this with iterator
                    for entry in &bucket.entries {
                        entry.infixes(key, node_end_depth, start_depth, end_depth, f, out);
                    }
                }
            }

            pub(super) unsafe fn has_prefix(
                node: *const Self,
                at_depth: usize,
                key: &[u8; KEY_LEN],
                end_depth: usize,
            ) -> bool {
                let node_end_depth = ((*node).end_depth as usize);
                let leaf_key: &[u8; KEY_LEN] = &(*(*node).min).key;
                for depth in at_depth..std::cmp::min(node_end_depth, end_depth) {
                    let key_depth = O::key_index(depth);
                    if leaf_key[key_depth] != key[key_depth] {
                        return false;
                    }
                }
                if end_depth < node_end_depth {
                    return true;
                }
                if let Some(child) = (*node).child_table.get(key[O::key_index(node_end_depth)]) {
                    return child.has_prefix(node_end_depth, key, end_depth);
                }
                return false;
            }

            pub(super) unsafe fn segmented_len(
                node: *const Self,
                at_depth: usize,
                key: &[u8; KEY_LEN],
                start_depth: usize,
            ) -> u64 {
                let node_end_depth = ((*node).end_depth as usize);
                let leaf_key: &[u8; KEY_LEN] = &(*(*node).min).key;
                for depth in at_depth..std::cmp::min(node_end_depth, start_depth) {
                    let key_depth = O::key_index(depth);
                    if leaf_key[key_depth] != key[key_depth] {
                        return 0;
                    }
                }
                if start_depth <= node_end_depth {
                    if S::segment(O::key_index(start_depth))
                        != S::segment(O::key_index(node_end_depth))
                    {
                        return 1;
                    } else {
                        return (*node).segment_count;
                    }
                }
                if let Some(child) = (*node).child_table.get(key[O::key_index(node_end_depth)]) {
                    return child.segmented_len(node_end_depth, key, start_depth);
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
        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>, V: Clone>
            $name<KEY_LEN, O, S, V>
        {
            pub(super) unsafe fn grow(head: &mut Head<KEY_LEN, O, S, V>) {
                debug_assert!(head.tag() == HeadTag::$name);
                unsafe {
                    let node: *const Self = head.ptr();
                    let layout = Layout::new::<$grown_name<KEY_LEN, O, S, V>>();
                    let ptr = alloc(layout) as *mut $grown_name<KEY_LEN, O, S, V>;
                    if ptr.is_null() {
                        panic!("Allocation failed!");
                    }
                    std::ptr::write(
                        ptr,
                        $grown_name::<KEY_LEN, O, S, V> {
                            key_ordering: PhantomData,
                            key_segments: PhantomData,
                            rc: atomic::AtomicU32::new(1),
                            end_depth: (*node).end_depth,
                            leaf_count: (*node).leaf_count,
                            segment_count: (*node).segment_count,
                            min: (*node).min,
                            hash: (*node).hash,
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

pub(super) fn branch_for_size<
    const KEY_LEN: usize,
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
    V: Clone
>(
    n: usize,
    end_depth: usize,
) -> Head<KEY_LEN, O, S, V> {
    match n {
        1..=2 => unsafe {
            Head::new(
                HeadTag::Branch2,
                0,
                Branch2::<KEY_LEN, O, S, V>::new(end_depth),
            )
        },
        3..=4 => unsafe {
            Head::new(
                HeadTag::Branch4,
                0,
                Branch4::<KEY_LEN, O, S, V>::new(end_depth),
            )
        },
        5..=8 => unsafe {
            Head::new(
                HeadTag::Branch8,
                0,
                Branch8::<KEY_LEN, O, S, V>::new(end_depth),
            )
        },
        9..=16 => unsafe {
            Head::new(
                HeadTag::Branch16,
                0,
                Branch16::<KEY_LEN, O, S, V>::new(end_depth),
            )
        },
        17..=32 => unsafe {
            Head::new(
                HeadTag::Branch32,
                0,
                Branch32::<KEY_LEN, O, S, V>::new(end_depth),
            )
        },
        33..=64 => unsafe {
            Head::new(
                HeadTag::Branch64,
                0,
                Branch64::<KEY_LEN, O, S, V>::new(end_depth),
            )
        },
        65..=128 => unsafe {
            Head::new(
                HeadTag::Branch128,
                0,
                Branch128::<KEY_LEN, O, S, V>::new(end_depth),
            )
        },
        129..=256 => unsafe {
            Head::new(
                HeadTag::Branch256,
                0,
                Branch256::<KEY_LEN, O, S, V>::new(end_depth),
            )
        },
        _ => panic!("bad child count for branch"),
    }
}
