use super::*;
use std::alloc::{alloc, dealloc, Layout};
use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

fn min_key<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>>(
    l: *const Leaf<KEY_LEN>,
    r: *const Leaf<KEY_LEN>,
) -> *const Leaf<KEY_LEN> {
    if l.is_null() {
        return r;
    }
    if r.is_null() {
        return l;
    }
    unsafe {
        return if O::tree_ordered(&(*l).key) < O::tree_ordered(&(*r).key) { l } else { r };
    }
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
            pub min: *const Leaf<KEY_LEN>,
            pub leaf_count: u64,
            segment_count: u64,
            pub hash: u128,
            child_set: ByteBitset,
            child_table: $table<Head<KEY_LEN, O, S>>,
        }

        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            $name<KEY_LEN, O, S>
        {
            pub(super) fn new(end_depth: usize) -> *mut Self {
                unsafe {
                    let layout = Layout::new::<Self>();
                    let ptr = alloc(layout) as *mut Self;
                    if ptr.is_null() {
                        panic!("Allocation failed!");
                    }
                    std::ptr::write(ptr, Self {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: end_depth as u32,
                        min: std::ptr::null(),
                        leaf_count: 0,
                        segment_count: 0,
                        hash: 0,
                        child_set: ByteBitset::new_empty(),
                        child_table: $table::new(),
                    });

                    ptr
                }
            }

            pub(super) unsafe fn rc_inc(node: *mut Self) -> *mut Self {
                unsafe {
                    let mut current = (*node).rc.load(Relaxed);
                    loop {
                        if current == u32::MAX {
                            let layout = Layout::new::<Self>();
                            let ptr = alloc(layout) as *mut Self;
                            if ptr.is_null() {
                                panic!("Allocation failed!");
                            }
                            std::ptr::write(ptr, Self {
                                key_ordering: PhantomData,
                                key_segments: PhantomData,
                                rc: atomic::AtomicU32::new(1),
                                end_depth: (*node).end_depth,
                                min: std::ptr::null(),
                                leaf_count: (*node).leaf_count,
                                segment_count: (*node).segment_count,
                                hash: (*node).hash,
                                child_set: (*node).child_set,
                                child_table: (*node).child_table.clone(),
                            });

                            Self::reset_min(ptr);

                            return ptr;
                        }
                        match (*node).rc.compare_exchange(current, current + 1, Relaxed, Relaxed) {
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
        
                    let layout = Layout::new::<Self>();
                    let ptr = node as *mut u8;
                    dealloc(ptr, layout);
                }
            }

            pub(super) unsafe fn rc_mut(head: &mut Head<KEY_LEN, O, S>) -> *mut Self {
                unsafe {
                    let node: *const Self = head.ptr();
                    if (*node).rc.load(Acquire) != 1 {
                        let layout = Layout::new::<Self>();
                        let ptr = alloc(layout) as *mut Self;
                        if ptr.is_null() {
                            panic!("Allocation failed!");
                        }
                        std::ptr::write(ptr, Self {
                            key_ordering: PhantomData,
                            key_segments: PhantomData,
                            rc: atomic::AtomicU32::new(1),
                            end_depth: (*node).end_depth,
                            min: std::ptr::null(),
                            leaf_count: (*node).leaf_count,
                            segment_count: (*node).segment_count,
                            hash: (*node).hash,
                            child_set: (*node).child_set,
                            child_table: (*node).child_table.clone(),
                        });

                        Self::reset_min(ptr);

                        *head = Head::new(HeadTag::$name, head.key().unwrap(), ptr);
                    }
                    head.ptr()
                }
            }

            pub unsafe fn reset_min(node: *mut Self) {
                let min_key = (*node).child_set.find_first_set().expect("must have childen");
                (*node).min = (*node).child_table.get(min_key).expect("min_key child must exist").min();
            }

            pub unsafe fn count(node: *const Self) -> u64 {
                (*node).leaf_count
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

            pub unsafe fn insert(
                node: *mut Self,
                child: Head<KEY_LEN, O, S>,
                child_hash: u128,
            ) -> Head<KEY_LEN, O, S> {
                if let Some(byte_key) = child.key() {
                    let end_depth = (*node).end_depth as usize;
                    (*node).min = std::ptr::null();
                    (*node).child_set.set(byte_key);
                    (*node).leaf_count += child.count();
                    (*node).segment_count += child.count_segment(end_depth);
                    (*node).hash ^= child_hash;
                    (*node).child_table.put(child)
                } else {
                    Head::empty()
                }
            }

            pub unsafe fn reinsert(
                node: *mut Self,
                child: Head<KEY_LEN, O, S>,
            ) -> Head<KEY_LEN, O, S> {
                (*node).child_table.put(child)
            }

            pub unsafe fn peek(node: *const Self, at_depth: usize) -> Peek {
                if at_depth == (*node).end_depth as usize {
                    Peek::Branch((*node).child_set)
                } else {
                    Peek::Fragment(Leaf::<KEY_LEN>::peek::<O>((*node).min, at_depth))
                }
            }

            pub unsafe fn branch(node: *const Self, key: u8) -> Head<KEY_LEN, O, S> {
                (*node).child_table.get(key).expect("no such child").clone()
            }

            pub unsafe fn with_start(
                node: *mut Self,
                new_start_depth: usize,
            ) -> Head<KEY_LEN, O, S> {
                Head::new(
                    HeadTag::$name,
                    Leaf::<KEY_LEN>::peek::<O>((*node).min, new_start_depth),
                    node,
                )
            }

            pub unsafe fn put(
                head: &mut Head<KEY_LEN, O, S>,
                entry: &Entry<KEY_LEN>,
                start_depth: usize,
            ) {
                let mut depth = start_depth;
                let node: *const Self = head.ptr();
                loop {
                    let key_byte = entry.peek::<O>(depth);
                    match Self::peek(node, depth) {
                        Peek::Fragment(byte) if byte == key_byte => depth += 1,
                        Peek::Fragment(_) => {
                            // The key diverged from what we already have, so we need to introduce
                            // a branch at the discriminating depth.

                            let new_branch = Branch4::new(depth);
                            Branch4::insert(new_branch, entry.leaf(depth), entry.hash);
                            Branch4::insert(new_branch, head.with_start(depth), head.hash());

                            Branch4::reset_min(new_branch);

                            *head = Branch4::with_start(new_branch, start_depth);
                            return;
                        }
                        Peek::Branch(children) if children.is_set(key_byte) => {
                            // We already have a child with the same byte as the key.

                            let inner = Self::rc_mut(head);
                            let child = (*inner)
                                .child_table
                                .get_mut(key_byte)
                                .expect("table content should match child set content");

                            let old_child_hash = child.hash();
                            let old_child_segment_count = child.count_segment(depth);
                            let old_child_leaf_count = child.count();

                            child.put(entry, depth);

                            (*inner).hash = ((*inner).hash ^ old_child_hash) ^ child.hash();
                            (*inner).segment_count = ((*inner).segment_count
                                - old_child_segment_count)
                                + child.count_segment(depth);
                            (*inner).leaf_count =
                                ((*inner).leaf_count - old_child_leaf_count) + child.count();

                            return;
                        }
                        Peek::Branch(_) => {
                            // We don't have a child with the byte of the key.

                            let inner = Self::rc_mut(head);
                            let mut displaced = Self::insert(inner, entry.leaf(depth), entry.hash);

                            while None != displaced.key() {
                                head.grow();
                                displaced = head.reinsert(displaced);
                            }
                            head.reset_min();
                            return;
                        }
                    }
                }
            }

            pub(super) unsafe fn infixes<const INFIX_LEN: usize, F>(
                node: *const Self,
                key: [u8; KEY_LEN],
                at_depth: usize,
                start_depth: usize,
                end_depth: usize,
                f: F,
                out: &mut Vec<[u8; INFIX_LEN]>,
            ) where
                F: Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN] + Copy,
            {
                for depth in at_depth..((*node).end_depth as usize) {
                    if start_depth <= depth {
                        if end_depth < (*node).end_depth as usize {
                            out.push(f((*(*node).min).key));
                        } else {
                            for child in (*node).child_set {
                                Self::branch(node, child).infixes(
                                    key,
                                    depth,
                                    start_depth,
                                    end_depth,
                                    f,
                                    out,
                                );
                            }
                        }
                        return;
                    }
                    if Leaf::peek::<O>((*node).min, depth) != key[depth] {
                        return;
                    }
                }
                for child in (*node).child_set {
                    Self::branch(node, child).infixes(
                        key,
                        (*node).end_depth as usize,
                        start_depth,
                        end_depth,
                        f,
                        out,
                    );
                }
            }

            pub(super) unsafe fn has_prefix(
                node: *const Self,
                at_depth: usize,
                key: [u8; KEY_LEN],
                end_depth: usize,
            ) -> bool {
                let node_end_depth = ((*node).end_depth as usize);
                for depth in at_depth..node_end_depth {
                    if end_depth < depth {
                        return true;
                    }
                    if Leaf::peek::<O>((*node).min, depth) != key[depth] {
                        return false;
                    }
                }
                return Self::branch(node, key[node_end_depth]).has_prefix(
                    node_end_depth,
                    key,
                    end_depth,
                );
            }

            pub(super) unsafe fn segmented_len(
                node: *const Self,
                depth: usize,
                key: [u8; KEY_LEN],
                start_depth: usize,
            ) -> usize {
                let mut depth = depth;
                loop {
                    if start_depth <= depth {
                        if S::segment(O::key_index(start_depth))
                            != S::segment(O::key_index((*node).end_depth as usize))
                        {
                            return 1;
                        } else {
                            return (*node).segment_count as usize;
                        }
                    }
                    match Self::peek(node, depth) {
                        Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                        Peek::Branch(children) if children.is_set(key[depth]) => {
                            return Self::branch(node, key[depth]).segmented_len(
                                depth,
                                key,
                                start_depth,
                            );
                        }
                        _ => {
                            return 0;
                        }
                    }
                }
            }
        }
    };
}

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
                unsafe {
                    let node: *const Self = head.ptr();
                    let layout = Layout::new::<$grown_name<KEY_LEN, O, S>>();
                    let ptr = alloc(layout) as *mut $grown_name<KEY_LEN, O, S>;
                    if ptr.is_null() {
                        panic!("Allocation failed!");
                    }
                    std::ptr::write(ptr, $grown_name::<KEY_LEN, O, S> {
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        rc: atomic::AtomicU32::new(1),
                        end_depth: (*node).end_depth,
                        leaf_count: (*node).leaf_count,
                        segment_count: (*node).segment_count,
                        min: std::ptr::null(),
                        hash: (*node).hash,
                        child_set: (*node).child_set,
                        child_table: (*node).child_table.grow(),
                    });

                    *head = Head::new(HeadTag::$grown_name, head.key().unwrap(), ptr);
                }
            }
        }
    };
}

create_grow!(Branch4, Branch8);
create_grow!(Branch8, Branch16);
create_grow!(Branch16, Branch32);
create_grow!(Branch32, Branch64);
create_grow!(Branch64, Branch128);
create_grow!(Branch128, Branch256);
