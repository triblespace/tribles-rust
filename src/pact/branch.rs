use super::*;

macro_rules! create_branch {
    ($name:ident, $body_name:ident, $table:tt) => {
        #[derive(Clone, Debug)]
        #[repr(C)]
        pub(super) struct $body_name<
            const KEY_LEN: usize,
            O: KeyOrdering<KEY_LEN>,
            S: KeySegmentation<KEY_LEN>,
        > {
            leaf_count: u32,
            segment_count: u32,
            hash: u128,
            child_set: ByteBitset,
            key: [u8; KEY_LEN],
            key_ordering: PhantomData<O>,
            key_segments: PhantomData<S>,
            child_table: $table<Head<KEY_LEN, O, S>>,
        }

        #[derive(Clone, Debug)]
        #[repr(C)]
        pub(super) struct $name<
            const KEY_LEN: usize,
            O: KeyOrdering<KEY_LEN>,
            S: KeySegmentation<KEY_LEN>,
        > {
            tag: HeadTag,
            start_depth: u8,
            fragment: [u8; HEAD_FRAGMENT_LEN],
            end_depth: u8,
            body: Arc<$body_name<KEY_LEN, O, S>>,
            key_ordering: PhantomData<O>,
            key_segments: PhantomData<S>,
        }

        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            From<$name<KEY_LEN, O, S>> for Head<KEY_LEN, O, S>
        {
            fn from(head: $name<KEY_LEN, O, S>) -> Self {
                unsafe { transmute(head) }
            }
        }

        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            $name<KEY_LEN, O, S>
        {
            pub(super) fn new(start_depth: usize, end_depth: usize, key: &[u8; KEY_LEN]) -> Self {
                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                copy_start(fragment.as_mut_slice(), &key[..], start_depth);

                Self {
                    tag: HeadTag::$name,
                    start_depth: start_depth as u8,
                    fragment: fragment,
                    end_depth: end_depth as u8,
                    key_ordering: PhantomData,
                    key_segments: PhantomData,
                    body: Arc::new($body_name {
                        leaf_count: 0,
                        segment_count: 0,
                        key: *key,
                        hash: 0,
                        child_set: ByteBitset::new_empty(),
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        child_table: $table::new(),
                    }),
                }
            }
        }

        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            HeadVariant<KEY_LEN, O, S> for $name<KEY_LEN, O, S>
        {
            fn count(&self) -> u32 {
                self.body.leaf_count
            }

            fn count_segment(&self, at_depth: usize) -> u32 {
                if S::segment(O::key_index(at_depth))
                    != S::segment(O::key_index(self.end_depth as usize))
                {
                    1
                } else {
                    self.body.segment_count
                }
            }

            fn insert(&mut self, child: Head<KEY_LEN, O, S>) -> Head<KEY_LEN, O, S> {
                if let Some(byte_key) = child.key() {
                    let body = Arc::make_mut(&mut self.body);
                    body.child_set.set(byte_key);
                    body.leaf_count += child.count();
                    body.segment_count += child.count_segment(self.end_depth as usize);
                    body.hash ^= child.hash();
                    body.child_table.put(child)
                } else {
                    Empty::new().into()
                }
            }

            fn reinsert(&mut self, child: Head<KEY_LEN, O, S>) -> Head<KEY_LEN, O, S> {
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

            fn child(&self, at_depth: usize, key: u8) -> Head<KEY_LEN, O, S> {
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

            fn hash(&self) -> u128 {
                self.body.hash
            }

            fn with_start(&self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                copy_start(fragment.as_mut_slice(), &self.body.key[..], new_start_depth);

                Head::from(Self {
                    tag: HeadTag::$name,
                    start_depth: new_start_depth as u8,
                    fragment,
                    end_depth: self.end_depth,
                    key_ordering: PhantomData,
                    key_segments: PhantomData,
                    body: Arc::clone(&self.body),
                })
            }

            fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Head<KEY_LEN, O, S> {
                let mut depth = self.start_depth as usize;
                loop {
                    let key_byte = key[O::key_index(depth)];
                    match self.peek(depth) {
                        Peek::Fragment(byte) if byte == key_byte => depth += 1,
                        Peek::Fragment(_) => {
                            // The key diverged from what we already have, so we need to introduce
                            // a branch at the discriminating depth.

                            let mut new_branch = Branch4::new(
                                self.start_depth as usize,
                                depth,
                                &O::tree_ordered(key),
                            );
                            new_branch.insert(Leaf::new(depth, key).into());
                            new_branch.insert(self.with_start(depth));

                            return Head::from(new_branch);
                        }
                        Peek::Branch(children) if children.is_set(key_byte) => {
                            // We already have a child with the same byte as the key.

                            let body = Arc::make_mut(&mut self.body);
                            let old_child = body
                                .child_table
                                .get_mut(key_byte)
                                .expect("table content should match child set content");
                            let old_child_hash = old_child.hash();

                            let old_child_segment_count = old_child.count_segment(depth);
                            let old_child_leaf_count = old_child.count();

                            let new_child = old_child.put(key);

                            body.hash = (body.hash ^ old_child_hash) ^ new_child.hash();

                            body.segment_count = (body.segment_count - old_child_segment_count)
                                + new_child.count_segment(depth);
                            body.leaf_count =
                                (body.leaf_count - old_child_leaf_count) + new_child.count();
                            body.child_table.put(new_child);

                            return self.clone().into();
                        }
                        Peek::Branch(_) => {
                            // We don't have a child with the byte of the key.

                            let mut displaced = self.insert(Leaf::new(depth, key).into());
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

            fn infixes<const INFIX_LEN: usize, F>(
                &self,
                key: [u8; KEY_LEN],
                start_depth: usize,
                end_depth: usize,
                f: F,
                out: &mut Vec<[u8; INFIX_LEN]>,
            ) where
                F: Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN] + Copy,
            {
                let mut depth = self.start_depth as usize;
                loop {
                    if start_depth <= depth {
                        if end_depth < self.end_depth as usize {
                            out.push(f(O::key_ordered(&self.body.key)));
                        } else {
                            for child in self.body.child_set {
                                self.child(self.end_depth as usize, child).infixes(
                                    key,
                                    start_depth,
                                    end_depth,
                                    f,
                                    out,
                                );
                            }
                        }
                        return;
                    }
                    match self.peek(depth) {
                        Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                        Peek::Fragment(_) => return,
                        Peek::Branch(children) => {
                            for child in children {
                                self.child(depth, child).infixes(
                                    key,
                                    start_depth,
                                    end_depth,
                                    f,
                                    out,
                                );
                            }
                        }
                    }
                }
            }

            fn has_prefix(&self, key: [u8; KEY_LEN], end_depth: usize) -> bool {
                let mut depth = self.start_depth as usize;
                loop {
                    if end_depth < depth {
                        return true;
                    }
                    match self.peek(depth) {
                        Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                        Peek::Fragment(_) => return false,
                        Peek::Branch(children) => {
                            self.child(depth, key[depth]).has_prefix(key, end_depth);
                        }
                    }
                }
            }

            fn segmented_len(&self, key: [u8; KEY_LEN], start_depth: usize) -> usize {
                let mut depth = self.start_depth as usize;
                loop {
                    if start_depth <= depth {
                        if S::segment(O::key_index(start_depth))
                            != S::segment(O::key_index(self.end_depth as usize))
                        {
                            return 1;
                        } else {
                            return self.body.segment_count as usize;
                        }
                    }
                    match self.peek(depth) {
                        Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                        Peek::Fragment(_) => return 0,
                        Peek::Branch(children) => {
                            return self
                                .child(depth, key[depth])
                                .segmented_len(key, start_depth);
                        }
                    }
                }
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
        impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
            $name<KEY_LEN, O, S>
        {
            pub(super) fn grow(&self) -> Head<KEY_LEN, O, S> {
                Head::<KEY_LEN, O, S>::from($grown_name {
                    tag: HeadTag::$grown_name,
                    start_depth: self.start_depth,
                    fragment: self.fragment,
                    end_depth: self.end_depth,
                    key_ordering: PhantomData,
                    key_segments: PhantomData,
                    body: Arc::new($grown_body_name {
                        leaf_count: self.body.leaf_count,
                        segment_count: self.body.segment_count,
                        hash: self.body.hash,
                        child_set: self.body.child_set,
                        key: self.body.key,
                        key_ordering: PhantomData,
                        key_segments: PhantomData,
                        child_table: self.body.child_table.grow(),
                    }),
                })
            }
        }
    };
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Branch256<KEY_LEN, O, S>
{
    pub(super) fn grow(&self) -> Head<KEY_LEN, O, S> {
        panic!("`grow` called on Branch256");
    }
}

create_grow!(Branch4, Branch8, BranchBody8);
create_grow!(Branch8, Branch16, BranchBody16);
create_grow!(Branch16, Branch32, BranchBody32);
create_grow!(Branch32, Branch64, BranchBody64);
create_grow!(Branch64, Branch128, BranchBody128);
create_grow!(Branch128, Branch256, BranchBody256);
