use super::*;
use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};

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
            min: Head<KEY_LEN, O, S>,
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
                    min: Head::empty(),
                    leaf_count: 0,
                    segment_count: 0,
                    hash: 0,
                    child_set: ByteBitset::new_empty(),
                    child_table: $table::new(),
                };

                ptr
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
                    Peek::Fragment((*node).key[O::key_index(at_depth)])
                }
            }

            pub fn branch(node: *const Self, key: u8) -> Head<KEY_LEN, O, S> {
                (*node).child_table
                    .get(key)
                    .expect("no such child")
                    .clone()
            }

            pub fn hash(node: *const Self) -> u128 {
                (*node).hash
            }

            pub fn with_start(node: *const Self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
                Head::new(HeadTag::$name, (*node).key[O::key_index(new_start_depth)], node.rc_inc())
            }

            pub fn put(node: *mut Self, key: &SharedKey<KEY_LEN>, start_depth: usize) -> Head<KEY_LEN, O, S> {
                let mut depth = start_depth;
                loop {
                    let key_byte = key[O::key_index(depth)];
                    match Self::peek(node, depth) {
                        Peek::Fragment(byte) if byte == key_byte => depth += 1,
                        Peek::Fragment(_) => {
                            // The key diverged from what we already have, so we need to introduce
                            // a branch at the discriminating depth.

                            let mut new_branch = Branch4::new(depth);
                            new_branch.insert(Leaf::new(depth, key).into());
                            new_branch.insert(Self::with_start(node, depth));

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

                            let new_child = old_child.put(key, depth);

                            mutable.hash = (mutable.hash ^ old_child_hash) ^ new_child.hash();

                            mutable.segment_count = (mutable.segment_count - old_child_segment_count)
                                + new_child.count_segment(depth);
                                mutable.leaf_count =
                                (mutable.leaf_count - old_child_leaf_count) + new_child.count();
                            mutable.child_table.put(new_child);

                            return mutable.with_start(start_depth);
                        }
                        Peek::Branch(_) => {
                            // We don't have a child with the byte of the key.

                            let mutable = Self::rc_mut(node);
                            let mut displaced = mutable.insert(Leaf::new(depth, key).into());
                            if None == displaced.key() {
                                return mutable.with_start(start_depth);
                            }

                            let mut new_head = mutable.with_start(start_depth);
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
                depth: usize,
                start_depth: usize,
                end_depth: usize,
                f: F,
                out: &mut Vec<[u8; INFIX_LEN]>,
            ) where
                F: Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN] + Copy,
            {
                let mut depth = depth;
                loop {
                    if start_depth <= depth {
                        if end_depth < (*node).end_depth as usize {
                            out.push(f(*(*node).key));
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
                    match Self::peek(node, depth) {
                        Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                        Peek::Fragment(_) => return,
                        Peek::Branch(children) => {
                            for child in children {
                                Self::branch(node, child).infixes(
                                    key,
                                    depth,
                                    start_depth,
                                    end_depth,
                                    f,
                                    out,
                                );
                            }
                            return;
                        }
                    }
                }
            }

            fn has_prefix(node: *const Self, depth: usize, key: [u8; KEY_LEN], end_depth: usize) -> bool {
                let mut depth = depth;
                loop {
                    if end_depth < depth {
                        return true;
                    }
                    match Self::peek(node, depth) {
                        Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                        Peek::Fragment(_) => return false,
                        Peek::Branch(_) => {
                            return Self::branch(node, key[depth]).has_prefix(depth, key, end_depth);
                        }
                    }
                }
            }

            fn segmented_len(node: *const Self, depth: usize, key: [u8; KEY_LEN], start_depth: usize) -> usize {
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
                            return Self::branch(node, key[depth])
                                    .segmented_len(depth, key, start_depth);
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
