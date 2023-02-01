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
        }
        impl<const KEY_LEN: usize> HeadVariant<KEY_LEN> for $name<KEY_LEN> {
            fn count(&self) -> u64 {
                self.body.child.count()
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
                    depth if depth == self.end_depth as usize => {
                        Peek::Branch(ByteBitset::new_singleton(
                            self.body.child.key().expect("child should exist"),
                        ))
                    }
                    depth if depth < self.start_depth as usize + self.fragment.len() => {
                        Peek::Fragment(self.fragment[index_start(self.start_depth as usize, depth)])
                    }
                    depth => Peek::Fragment(
                        self.body.fragment
                            [index_end(self.body.fragment.len(), self.end_depth as usize, depth)],
                    ),
                }
            }

            fn get(&self, at_depth: usize, key: u8) -> Head<KEY_LEN> {
                match self.peek(at_depth) {
                    Peek::Fragment(byte) if byte == key => self.clone().into(),
                    Peek::Branch(children) if children.is_set(key) => self.body.child.clone(),
                    _ => Empty::new().into(),
                }
            }

            fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let mut depth = self.start_depth as usize;
                loop {
                    match self.peek(depth) {
                        Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                        Peek::Fragment(_) => {
                            // The key diverged from what we already have, so we need to introduce
                            // a branch at the discriminating depth.
                            let sibling_leaf =
                                Head::<KEY_LEN>::from(Leaf::new(depth, key)).wrap_path(depth, key);

                            let mut new_branch =
                                Branch4::new(self.start_depth as usize, depth, key);

                            new_branch.insert(key, sibling_leaf);
                            new_branch.insert(
                                key,
                                Head::<KEY_LEN>::from(self.clone()).wrap_path(depth, key),
                            );

                            return Head::<KEY_LEN>::from(new_branch)
                                .wrap_path(self.start_depth as usize, key);
                        }
                        Peek::Branch(_) => {
                            // The entire fragment matched with the key.
                            let new_body = Arc::make_mut(&mut self.body);
                            return new_body
                                .child
                                .put(key)
                                .wrap_path(self.start_depth as usize, key);
                        }
                    }
                }
            }

            fn hash(&self, prefix: &[u8; KEY_LEN]) -> u128 {
                let mut key = *prefix;

                for depth in self.start_depth as usize..self.end_depth as usize {
                    match self.peek(depth) {
                        Peek::Fragment(byte) => key[depth] = byte,
                        _ => panic!(),
                    }
                }

                return self.body.child.hash(&key);
            }

            fn with_start_depth(
                &self,
                new_start_depth: usize,
                key: &[u8; KEY_LEN],
            ) -> Head<KEY_LEN> {
                let actual_start_depth = max(
                    new_start_depth as isize,
                    self.end_depth as isize
                        - (self.body.fragment.len() as isize + HEAD_FRAGMENT_LEN as isize),
                ) as usize;

                let mut new_fragment = [0; HEAD_FRAGMENT_LEN];
                for i in 0..new_fragment.len() {
                    let depth = actual_start_depth + i;

                    new_fragment[i] = if depth < self.start_depth as usize {
                        key[depth]
                    } else {
                        match self.peek(depth) {
                            Peek::Fragment(byte) => byte,
                            Peek::Branch(_) => break,
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
        }
    };
}

create_path!(Path14, PathBody14, 14);
create_path!(Path30, PathBody30, 30);
create_path!(Path46, PathBody46, 46);
create_path!(Path62, PathBody62, 62);
