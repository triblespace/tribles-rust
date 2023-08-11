use super::*;
use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};

fn min_key<const KEY_LEN: usize>(
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
        return if (*l).key < (*r).key { l } else { r };
    }
}

macro_rules! create_branch {
    ($name:ident, $table:tt) => {
        #[derive(Clone, Debug)]
        #[repr(C)]
        pub(super) struct $name<
            const KEY_LEN: usize,
            O: KeyOrdering<KEY_LEN>,
            S: KeySegmentation<KEY_LEN>,
        > {
            key_ordering: PhantomData<O>,
            key_segments: PhantomData<S>,

            rc: u32,
            end_depth: u32,
            pub min: *const Leaf<KEY_LEN>,
            leaf_count: u64,
            segment_count: u64,
            hash: u128,
            child_set: ByteBitset,
            child_table: $table<Head<KEY_LEN, O, S>>,
        }

        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            $name<KEY_LEN, O, S>
        {
            pub(super) fn new(end_depth: usize) -> *mut Self {
                let layout = Layout::new::<Self>();
                let ptr = alloc(layout) as *mut Self;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                *ptr = Self {
                    key_ordering: PhantomData,
                    key_segments: PhantomData,
                    rc: 1,
                    end_depth: end_depth as u32,
                    min: std::ptr::null_mut(),
                    leaf_count: 0,
                    segment_count: 0,
                    hash: 0,
                    child_set: ByteBitset::new_empty(),
                    child_table: $table::new(),
                };

                ptr
            }

            pub(super) fn rc_inc(node: *mut Self) -> *mut Self {
                //TODO copy on overflow
                unsafe {
                    (*node).rc = (*node).rc + 1;
                    node
                }
            }

            pub(super) fn rc_dec(node: *mut Self) {
                unsafe {
                    if (*node).rc == 1 {
                        let layout = Layout::new::<Self>();
                        let ptr = node as *mut u8;
                        dealloc(ptr, layout);
                    } else {
                        (*node).rc = (*node).rc - 1
                    }
                }
            }

            pub(super) fn rc_mut(node: *mut Self) -> *mut Self {
                unsafe {
                    if (*node).rc == 1 {
                        node
                    } else {
                        let layout = Layout::new::<Self>();
                        let ptr = alloc(layout) as *mut Self;
                        if ptr.is_null() {
                            panic!("Allocation failed!");
                        }
                        *ptr = Self {
                            key_ordering: PhantomData,
                            key_segments: PhantomData,
                            rc: 1,
                            end_depth: (*node).end_depth,
                            min: (*node).min,
                            leaf_count: (*node).leaf_count,
                            segment_count: (*node).segment_count,
                            hash: (*node).hash,
                            child_set: (*node).child_set,
                            child_table: (*node).child_table,
                        };

                        ptr
                    }
                }
            }

            pub fn count(node: *const Self) -> u64 {
                (*node).leaf_count
            }

            pub fn count_segment(node: *const Self, at_depth: usize) -> u64 {
                if S::segment(O::key_index(at_depth))
                    != S::segment(O::key_index((*node).end_depth as usize))
                {
                    1
                } else {
                    (*node).segment_count
                }
            }

            pub fn insert(node: *mut Self, child: Head<KEY_LEN, O, S>) -> Head<KEY_LEN, O, S> {
                if let Some(byte_key) = child.key() {
                    let end_depth = (*node).end_depth as usize;
                    (*node).min = min_key((*node).min, child.min());
                    (*node).child_set.set(byte_key);
                    (*node).leaf_count += child.count();
                    (*node).segment_count += child.count_segment(end_depth);
                    (*node).hash ^= child.hash();
                    (*node).child_table.put(child)
                } else {
                    Head::empty()
                }
            }

            pub fn reinsert(node: *mut Self, child: Head<KEY_LEN, O, S>) -> Head<KEY_LEN, O, S> {
                (*node).child_table.put(child)
            }

            pub fn peek(node: *const Self, at_depth: usize) -> Peek {
                if at_depth == (*node).end_depth as usize {
                    Peek::Branch((*node).child_set)
                } else {
                    Peek::Fragment(Leaf::<KEY_LEN>::peek::<O>((*node).min, at_depth))
                }
            }

            pub fn branch(node: *const Self, key: u8) -> Head<KEY_LEN, O, S> {
                (*node).child_table.get(key).expect("no such child").clone()
            }

            pub fn hash(node: *const Self) -> u128 {
                (*node).hash
            }

            pub fn with_start(node: *const Self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
                Head::new(
                    HeadTag::$name,
                    (*node).key[O::key_index(new_start_depth)],
                    node.rc_inc(),
                )
            }

            pub fn put(
                node: *mut Self,
                entry: &Entry<KEY_LEN>,
                start_depth: usize,
            ) -> Head<KEY_LEN, O, S> {
                let mut depth = start_depth;
                loop {
                    let key_byte = entry.peek::<O>(depth);
                    match Self::peek(node, depth) {
                        Peek::Fragment(byte) if byte == key_byte => depth += 1,
                        Peek::Fragment(_) => {
                            // The key diverged from what we already have, so we need to introduce
                            // a branch at the discriminating depth.

                            let mut new_branch = Branch4::new(depth);
                            Branch4::insert(new_branch, entry.leaf(depth));
                            Branch4::insert(new_branch, Self::with_start(node, depth));

                            return Branch4::with_start(new_branch, start_depth);
                        }
                        Peek::Branch(children) if children.is_set(key_byte) => {
                            // We already have a child with the same byte as the key.

                            let mutable = Self::rc_mut(node);
                            let old_child = mutable
                                .child_table
                                .get_mut(key_byte)
                                .expect("table content should match child set content");
                            let old_child_hash = old_child.hash();

                            let old_child_segment_count = old_child.count_segment(depth);
                            let old_child_leaf_count = old_child.count();

                            let new_child = old_child.put(entry, depth);

                            mutable.hash = (mutable.hash ^ old_child_hash) ^ new_child.hash();

                            mutable.segment_count = (mutable.segment_count
                                - old_child_segment_count)
                                + new_child.count_segment(depth);
                            mutable.leaf_count =
                                (mutable.leaf_count - old_child_leaf_count) + new_child.count();
                            mutable.child_table.put(new_child);

                            return mutable.with_start(start_depth);
                        }
                        Peek::Branch(_) => {
                            // We don't have a child with the byte of the key.

                            let mutable = Self::rc_mut(node);
                            let mut displaced = Self::insert(mutable, entry.leaf(depth));

                            let mut new_head = Self::with_start(mutable, start_depth);

                            if None == displaced.key() {
                                return new_head;
                            }

                            while None != displaced.key() {
                                new_head = new_head.grow();
                                displaced = new_head.reinsert(displaced);
                            }
                            return new_head;
                        }
                    }
                }
            }

            fn infixes<const INFIX_LEN: usize, F>(
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

            fn has_prefix(
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

            fn segmented_len(
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
                        Peek::Fragment(_) => return 0,
                        Peek::Branch(children) => {
                            return Self::branch(node, key[depth]).segmented_len(
                                depth,
                                key,
                                start_depth,
                            );
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
            pub(super) fn grow(node: *const $name<KEY_LEN, O, S>, key: u8) -> Head<KEY_LEN, O, S> {
                let layout = Layout::new::<$grown_name<KEY_LEN, O, S>>();
                let ptr = alloc(layout) as *mut $grown_name<KEY_LEN, O, S>;
                if ptr.is_null() {
                    panic!("Allocation failed!");
                }
                *ptr = $grown_name::<KEY_LEN, O, S> {
                    key_ordering: PhantomData,
                    key_segments: PhantomData,
                    rc: 1,
                    end_depth: (*node).end_depth,
                    leaf_count: (*node).leaf_count,
                    segment_count: (*node).segment_count,
                    min: (*node).min.clone(),
                    hash: node.hash,
                    child_set: node.child_set,
                    child_table: node.child_table.grow(),
                };

                Head::new(HeadTag::$name, key, ptr)
            }
        }
    };
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Branch256<KEY_LEN, O, S>
{
    pub(super) fn grow(&self, key: u8) -> Head<KEY_LEN, O, S> {
        panic!("`grow` called on Branch256");
    }
}

create_grow!(Branch4, Branch8);
create_grow!(Branch8, Branch16);
create_grow!(Branch16, Branch32);
create_grow!(Branch32, Branch64);
create_grow!(Branch64, Branch128);
create_grow!(Branch128, Branch256);
