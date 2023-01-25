use super::*;

macro_rules! create_branch {
    ($name:ident, $body_name:ident, $table:tt) => {
        #[derive(Clone, Debug)]
        #[repr(C)]
        pub(super) struct $body_name<const KEY_LEN: usize> {
            leaf_count: u64,
            //rc: AtomicU16,
            //segment_count: u32, //TODO: increase this to a u48
            //hash: u128,
            child_set: ByteBitset,
            child_table: $table<Head<KEY_LEN>>,
        }

        #[derive(Clone, Debug)]
        #[repr(C)]
        pub(super) struct $name<const KEY_LEN: usize> {
            tag: HeadTag,
            start_depth: u8,
            fragment: [u8; HEAD_FRAGMENT_LEN],
            end_depth: u8,
            body: Arc<$body_name<KEY_LEN>>,
        }

        impl<const KEY_LEN: usize> From<$name<KEY_LEN>> for Head<KEY_LEN> {
            fn from(head: $name<KEY_LEN>) -> Self {
                unsafe { transmute(head) }
            }
        }

        impl<const KEY_LEN: usize> $name<KEY_LEN> {
            pub(super) fn new(start_depth: usize, end_depth: usize, key: &[u8; KEY_LEN]) -> Self {
                let actual_start_depth = max(
                    start_depth as isize,
                    end_depth as isize - HEAD_FRAGMENT_LEN as isize,
                ) as usize;

                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

                Self {
                    tag: HeadTag::$name,
                    start_depth: actual_start_depth as u8,
                    fragment: fragment,
                    end_depth: end_depth as u8,
                    body: Arc::new($body_name {
                        leaf_count: 0,
                        //rc: AtomicU16::new(1),
                        //segment_count: 0,
                        //node_hash: 0,
                        child_set: ByteBitset::new_empty(),
                        child_table: $table::new(),
                    }),
                }
            }

            pub(super) fn count(&self) -> u64 {
                self.body.leaf_count
            }

            pub(super) fn insert(&mut self, child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                let inner = Arc::make_mut(&mut self.body);
                inner
                    .child_set
                    .set(child.key().expect("leaf should have a byte key"));
                inner.leaf_count += child.count();
                inner.child_table.put(child)
            }

            pub(super) fn reinsert(&mut self, child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                let inner = Arc::make_mut(&mut self.body);
                inner.child_table.put(child)
            }

            pub(super) fn peek(&self, at_depth: usize) -> Option<u8> {
                if at_depth < self.start_depth as usize || self.end_depth as usize <= at_depth {
                    return None;
                }
                return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
            }

            pub(super) fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
                if at_depth == self.end_depth as usize {
                    *result_set = self.body.child_set;
                    return;
                }

                result_set.unset_all();
                if let Some(byte_key) = self.peek(at_depth) {
                    result_set.set(byte_key);
                    return;
                }
            }

            pub(super) fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let mut branch_depth = self.start_depth as usize;
                while Some(key[branch_depth]) == self.peek(branch_depth) {
                    branch_depth += 1;
                }

                if branch_depth == self.end_depth as usize {
                    // The entire fragment matched with the key.

                    let byte_key = key[branch_depth];
                    if self.body.child_set.is_set(byte_key) {
                        // We already have a child with the same byte as the key.

                        let new_body = Arc::make_mut(&mut self.body);
                        let old_child = new_body
                            .child_table
                            .get_mut(byte_key)
                            .expect("table content should match child set content");
                        //let old_child_hash = old_child.hash(key);
                        //let old_child_segment_count = old_child.segmentCount(branch_depth);
                        //let new_child_hash = new_child.hash(key);
                        let old_child_leaf_count = old_child.count();
                        let new_child = old_child.put(key);

                        //let new_hash = self.body.node_hash.update(old_child_hash, new_child_hash);
                        //let new_segment_count = self.body.segment_count - old_child_segment_count + new_child.segmentCount(branch_depth);

                        //new_body.node_hash = new_hash;
                        //new_body.segment_count = new_segment_count;
                        new_body.leaf_count = (new_body.leaf_count - old_child_leaf_count as u64)
                            + new_child.count() as u64;
                        new_body.child_table.put(new_child);

                        return self.clone().into();
                    } else {
                        // We don't have a child with the byte of the key.

                        let mut displaced = self.insert(
                            Head::from(Leaf::new(branch_depth, key)).wrap_path(branch_depth, key),
                        );
                        if None == displaced.key() {
                            Head::from(self.clone());
                        }

                        let mut new_self = Head::from(self.clone());
                        while None != displaced.key() {
                            new_self = new_self.grow();
                            displaced = new_self.reinsert(displaced);
                        }
                        return new_self;
                    }
                } else {
                    // The key diverged from what we already have, so we need to introduce
                    // a branch at the discriminating depth.

                    let sibling_leaf =
                        Head::from(Leaf::new(branch_depth, key)).wrap_path(branch_depth, key);

                    let mut new_branch = Branch4::new(self.start_depth as usize, branch_depth, key);
                    new_branch.insert(sibling_leaf);
                    new_branch
                        .insert(Head::<KEY_LEN>::from(self.clone()).wrap_path(branch_depth, key));

                    return Head::from(new_branch).wrap_path(self.start_depth as usize, key);
                }
            }

            pub(super) fn with_start_depth(
                &self,
                new_start_depth: usize,
                key: &[u8; KEY_LEN],
            ) -> Head<KEY_LEN> {
                let actual_start_depth = max(
                    new_start_depth as isize,
                    self.end_depth as isize - HEAD_FRAGMENT_LEN as isize,
                ) as usize;

                let mut new_fragment = [0; HEAD_FRAGMENT_LEN];
                for i in 0..new_fragment.len() {
                    let depth = actual_start_depth + i;
                    if (self.end_depth as usize <= depth) {
                        break;
                    }
                    new_fragment[i] = if depth < self.start_depth as usize {
                        println!("key @ {}", depth);
                        key[depth]
                    } else {
                        self.fragment[index_start(self.start_depth as usize, depth)]
                    }
                }
                Head::from(Self {
                    tag: HeadTag::$name,
                    start_depth: actual_start_depth as u8,
                    fragment: new_fragment,
                    end_depth: self.end_depth,
                    body: Arc::clone(&self.body),
                })
            }
        }
    };
}

create_branch!(Branch4, BranchBody4, ByteTable4);
create_branch!(Branch8, BranchBody8, ByteTable8);
create_branch!(Branch16, BranchBody16, ByteTable16);
create_branch!(Branch32, BranchBody32, ByteTable32);
create_branch!(Branch64, BranchBody64, ByteTable64);
create_branch!(Branch128, BranchBody128, ByteTable128);
create_branch!(Branch256, BranchBody256, ByteTable256);

macro_rules! create_grow {
    () => {};
    ($name:ident, $grown_name:ident, $grown_body_name:ident) => {
        impl<const KEY_LEN: usize> $name<KEY_LEN> {
            pub(super) fn grow(&self) -> Head<KEY_LEN> {
                Head::<KEY_LEN>::from($grown_name {
                    tag: HeadTag::$grown_name,
                    start_depth: self.start_depth,
                    fragment: self.fragment,
                    end_depth: self.end_depth,
                    body: Arc::new($grown_body_name {
                        leaf_count: self.body.leaf_count,
                        //segment_count: self.segment_count,
                        //node_hash: self.node_hash,
                        child_set: self.body.child_set,
                        child_table: self.body.child_table.grow(),
                    }),
                })
            }
        }
    };
}

impl<const KEY_LEN: usize> Branch256<KEY_LEN> {
    pub(super) fn grow(&self) -> Head<KEY_LEN> {
        panic!("`grow` called on Branch256");
    }
}

create_grow!(Branch4, Branch8, BranchBody8);
create_grow!(Branch8, Branch16, BranchBody16);
create_grow!(Branch16, Branch32, BranchBody32);
create_grow!(Branch32, Branch64, BranchBody64);
create_grow!(Branch64, Branch128, BranchBody128);
create_grow!(Branch128, Branch256, BranchBody256);
