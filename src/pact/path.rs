use super::*;

macro_rules! create_path {
    ($name:ident, $body_name:ident, $body_fragment_len:expr) => {
        #[derive(Clone, Debug)]
        #[repr(C)]
        pub(super) struct $body_name<const KEY_LEN: usize> {
            child: Head<KEY_LEN>,
            //rc: AtomicU16,
            fragment: [u8; $body_fragment_len],
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
            pub(super) fn new(
                start_depth: usize,
                key: &[u8; KEY_LEN],
                child: Head<KEY_LEN>,
            ) -> Self {
                let end_depth = child.start_depth();
                let mut body_fragment = [0; $body_fragment_len];
                copy_end(body_fragment.as_mut_slice(), &key[..], end_depth as usize);

                let path_body = Arc::new($body_name {
                    child: child,
                    //rc: AtomicU16::new(1),
                    fragment: body_fragment,
                });

                let actual_start_depth = max(
                    start_depth as isize,
                    end_depth as isize - ($body_fragment_len as isize + HEAD_FRAGMENT_LEN as isize),
                ) as usize;

                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

                Self {
                    tag: HeadTag::$name,
                    start_depth: actual_start_depth as u8,
                    fragment: fragment,
                    end_depth: end_depth,
                    body: path_body,
                }
            }

            pub(super) fn count(&self) -> u64 {
                self.body.child.count()
            }

            pub(super) fn peek(&self, at_depth: usize) -> Option<u8> {
                if at_depth < self.start_depth as usize || self.end_depth as usize <= at_depth {
                    return None;
                }
                if at_depth < self.start_depth as usize + self.fragment.len() {
                    return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
                }
                return Some(
                    self.body.fragment
                        [index_end(self.body.fragment.len(), self.end_depth as usize, at_depth)],
                );
            }

            pub(super) fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
                result_set.unset_all();
                if at_depth == self.end_depth as usize {
                    result_set.set(
                        self.body
                            .child
                            .peek(at_depth)
                            .expect("path child peek at child depth must succeed"),
                    );
                    return;
                }

                if let Some(byte_key) = self.peek(at_depth) {
                    result_set.set(byte_key);
                }
            }

            pub(super) fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let mut branch_depth = self.start_depth as usize;
                while Some(key[branch_depth]) == self.peek(branch_depth) {
                    branch_depth += 1;
                }

                if branch_depth == self.end_depth as usize {
                    // The entire fragment matched with the key.
                    let mut new_body = Arc::make_mut(&mut self.body);

                    let new_child = new_body.child.put(key);
                    if new_child.start_depth() != self.end_depth {
                        return new_child.wrap_path(self.start_depth as usize, key);
                    }

                    new_body.child = new_child;

                    return self.clone().into();
                } else {
                    // The key diverged from what we already have, so we need to introduce
                    // a branch at the discriminating depth.
                    let sibling_leaf = Head::<KEY_LEN>::from(Leaf::new(branch_depth, key))
                        .wrap_path(branch_depth, key);

                    let mut new_branch = Branch4::new(self.start_depth as usize, branch_depth, key);

                    new_branch.insert(sibling_leaf);
                    new_branch
                        .insert(Head::<KEY_LEN>::from(self.clone()).wrap_path(branch_depth, key));

                    return Head::<KEY_LEN>::from(new_branch)
                        .wrap_path(self.start_depth as usize, key);
                }
            }

            pub(super) fn with_start_depth(
                &self,
                new_start_depth: usize,
                key: &[u8; KEY_LEN],
            ) -> Head<KEY_LEN> {
                let actual_start_depth = max(
                    new_start_depth as isize,
                    self.end_depth as isize
                        - (self.body.fragment.len() as isize + HEAD_FRAGMENT_LEN as isize),
                ) as usize;

                let head_end_depth = self.start_depth as usize + HEAD_FRAGMENT_LEN;

                let mut new_fragment = [0; HEAD_FRAGMENT_LEN];
                for i in 0..new_fragment.len() {
                    let depth = actual_start_depth + i;
                    if (self.end_depth as usize <= depth) {
                        break;
                    }
                    new_fragment[i] = if (depth < self.start_depth as usize) {
                        key[depth]
                    } else {
                        if depth < head_end_depth {
                            self.fragment[index_start(self.start_depth as usize, depth)]
                        } else {
                            self.body.fragment[index_end(
                                self.body.fragment.len(),
                                self.end_depth as usize,
                                depth,
                            )]
                        }
                    }
                }

                Head::<KEY_LEN>::from(Self {
                    tag: HeadTag::$name,
                    start_depth: actual_start_depth as u8,
                    fragment: new_fragment,
                    end_depth: self.end_depth,
                    body: Arc::clone(&self.body),
                })
            }

            pub(super) fn insert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                panic!("`insert` called on path");
            }

            pub(super) fn reinsert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                panic!("`reinsert` called on path");
            }

            pub(super) fn grow(&self) -> Head<KEY_LEN> {
                panic!("`grow` called on path");
            }
        }
    };
}

create_path!(Path14, PathBody14, 14);
create_path!(Path30, PathBody30, 30);
create_path!(Path46, PathBody46, 46);
create_path!(Path62, PathBody62, 62);
