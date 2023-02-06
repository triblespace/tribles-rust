use super::*;

macro_rules! create_branch {
    ($name:ident, $body_name:ident, $table:tt) => {
        #[derive(Clone, Debug)]
        #[repr(C)]
        pub(super) struct $body_name<const KEY_LEN: usize> {
            leaf_count: u64,
            //rc: AtomicU16,
            //segment_count: u32, //TODO: increase this to a u48
            hash: u128,
            child_set: ByteBitset,
            key: [u8; KEY_LEN],
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
                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                fragment[..].copy_from_slice(&key[start_depth..start_depth + HEAD_FRAGMENT_LEN]);

                Self {
                    tag: HeadTag::$name,
                    start_depth: start_depth as u8,
                    fragment: fragment,
                    end_depth: end_depth as u8,
                    body: Arc::new($body_name {
                        leaf_count: 0,
                        //rc: AtomicU16::new(1),
                        //segment_count: 0,
                        key: *key,
                        hash: 0,
                        child_set: ByteBitset::new_empty(),
                        child_table: $table::new(),
                    }),
                }
            }
        }

        impl<const KEY_LEN: usize> HeadVariant<KEY_LEN> for $name<KEY_LEN> {
            fn count(&self) -> u64 {
                self.body.leaf_count
            }

            fn insert(&mut self, key: &[u8; KEY_LEN], child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                if let Some(byte_key) = child.key() {
                    let body = Arc::make_mut(&mut self.body);
                    body.child_set.set(byte_key);
                    body.leaf_count += child.count();
                    body.hash ^= child.hash(key);
                    body.child_table.put(child)
                } else {
                    Empty::new().into()
                }
            }

            fn reinsert(&mut self, child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                let inner = Arc::make_mut(&mut self.body);
                inner.child_table.put(child)
            }

            fn peek(&self, at_depth: usize) -> Peek {
                assert!(
                    self.start_depth as usize <= at_depth && at_depth <= self.end_depth as usize,
                    "Peek out of bounds: {} <= {} <= {}",
                    self.start_depth,
                    at_depth,
                    self.end_depth
                );
                match at_depth {
                    depth if depth == self.end_depth as usize => Peek::Branch(self.body.child_set),
                    depth if depth < self.start_depth as usize + self.fragment.len() => {
                        Peek::Fragment(self.fragment[index_start(self.start_depth as usize, depth)])
                    }
                    depth => Peek::Fragment(self.body.key[depth]),
                }
            }

            fn get(&self, at_depth: usize, key: u8) -> Head<KEY_LEN> {
                match self.peek(at_depth) {
                    Peek::Fragment(byte) if byte == key => self.clone().into(),
                    Peek::Branch(children) if children.is_set(key) => self
                        .body
                        .child_table
                        .get(key)
                        .expect("child table should match child set")
                        .clone(),
                    _ => Empty::new().into(),
                }
            }

            fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let mut depth = self.start_depth as usize;
                loop {
                    let key_byte = key[depth];
                    match self.peek(depth) {
                        Peek::Fragment(byte) if byte == key_byte => depth += 1,
                        Peek::Fragment(_) => {
                            // The key diverged from what we already have, so we need to introduce
                            // a branch at the discriminating depth.

                            let sibling_leaf = new_leaf(depth, key);

                            let mut new_branch =
                                Branch4::new(self.start_depth as usize, depth, key);
                            new_branch.insert(key, sibling_leaf);
                            new_branch.insert(key, self.clone().with_start(depth, key));

                            return Head::from(new_branch);
                        }
                        Peek::Branch(children) if children.is_set(key_byte) => {
                            // We already have a child with the same byte as the key.

                            let body = Arc::make_mut(&mut self.body);
                            let old_child = body
                                .child_table
                                .get_mut(key_byte)
                                .expect("table content should match child set content");
                            let old_child_hash = old_child.hash(key);
                            //let old_child_segment_count = old_child.segmentCount(depth);
                            let old_child_leaf_count = old_child.count();

                            let new_child = old_child.put(key);

                            body.hash = (body.hash ^ old_child_hash) ^ new_child.hash(key);
                            //let new_segment_count = self.body.segment_count - old_child_segment_count + new_child.segmentCount(depth);

                            //body.segment_count = new_segment_count;
                            body.leaf_count = (body.leaf_count - old_child_leaf_count as u64)
                                + new_child.count() as u64;
                            body.child_table.put(new_child);

                            return self.clone().into();
                        }
                        Peek::Branch(_) => {
                            // We don't have a child with the byte of the key.

                            let mut displaced = self.insert(key, new_leaf(depth, key));
                            if None == displaced.key() {
                                return Head::from(self.clone());
                            }

                            let mut new_self = Head::from(self.clone());
                            while None != displaced.key() {
                                new_self = new_self.grow();
                                displaced = new_self.reinsert(displaced);
                            }
                            return new_self;
                        }
                    }
                }
            }

            fn hash(&self, _prefix: &[u8; KEY_LEN]) -> u128 {
                self.body.hash
            }

            fn with_start(&self, new_start_depth: usize, _key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let mut new_fragment = [0; HEAD_FRAGMENT_LEN];
                new_fragment[..].copy_from_slice(
                    &self.body.key[new_start_depth..new_start_depth + HEAD_FRAGMENT_LEN],
                );

                Head::from(Self {
                    tag: HeadTag::$name,
                    start_depth: new_start_depth as u8,
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
                        hash: self.body.hash,
                        child_set: self.body.child_set,
                        key: self.body.key,
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
