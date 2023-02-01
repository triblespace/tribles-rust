use super::*;

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct Leaf<const KEY_LEN: usize> {
    _tag: HeadTag,
    start_depth: u8,
    fragment: [u8; LEAF_FRAGMENT_LEN],
}

impl<const KEY_LEN: usize> From<Leaf<KEY_LEN>> for Head<KEY_LEN> {
    fn from(head: Leaf<KEY_LEN>) -> Self {
        unsafe { transmute(head) }
    }
}

impl<const KEY_LEN: usize> Leaf<KEY_LEN> {
    pub(super) fn new(start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let actual_start_depth = max(start_depth, KEY_LEN - LEAF_FRAGMENT_LEN);

        let mut fragment = [0; LEAF_FRAGMENT_LEN];

        copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

        Self {
            _tag: HeadTag::Leaf,
            start_depth: actual_start_depth as u8,
            fragment: fragment,
        }
    }
}

impl<const KEY_LEN: usize> HeadVariant<KEY_LEN> for Leaf<KEY_LEN> {
    fn count(&self) -> u64 {
        1
    }

    fn peek(&self, at_depth: usize) -> Peek {
        assert!(
            self.start_depth as usize <= at_depth && at_depth < KEY_LEN as usize,
            "Peek out of bounds: {} <= {} < {}",
            self.start_depth,
            at_depth,
            KEY_LEN
        );

        Peek::Fragment(self.fragment[index_start(self.start_depth as usize, at_depth)])
    }

    fn get(&self, at_depth: usize, key: u8) -> Head<KEY_LEN> {
        match self.peek(at_depth) {
            Peek::Fragment(byte) if byte == key => self.clone().into(),
            _ => Empty::new().into(),
        }
    }

    fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        let mut depth = self.start_depth as usize;
        loop {
            if depth == KEY_LEN {
                return self.clone().into();
            }
            match self.peek(depth) {
                Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                Peek::Fragment(_) => {
                    let sibling_leaf = Head::<KEY_LEN>::from(Leaf::new(depth, key));

                    let mut new_branch = Branch4::new(self.start_depth as usize, depth, key);
                    new_branch.insert(key, sibling_leaf);
                    new_branch.insert(
                        key,
                        Head::<KEY_LEN>::from(self.clone()).wrap_path(depth, key),
                    );

                    return Head::<KEY_LEN>::from(new_branch)
                        .wrap_path(self.start_depth as usize, key);
                }
                Peek::Branch(_) => panic!(),
            }
        }
    }

    fn hash(&self, prefix: &[u8; KEY_LEN]) -> u128 {
        let mut key = *prefix;

        key[self.start_depth as usize..]
            .copy_from_slice(&self.fragment[..KEY_LEN - self.start_depth as usize]);

        let mut hasher = SipHasher24::new_with_key(unsafe { &SIP_KEY });
        hasher.write(&key[..]);
        return hasher.finish128().into();
    }

    fn with_start_depth(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        assert!(new_start_depth < KEY_LEN);

        let actual_start_depth = max(
            new_start_depth as isize,
            KEY_LEN as isize - (LEAF_FRAGMENT_LEN as isize),
        ) as usize;

        let mut new_fragment = [0; LEAF_FRAGMENT_LEN];
        for depth in actual_start_depth..KEY_LEN {
            if depth == KEY_LEN {
                break;
            }

            new_fragment[depth - actual_start_depth] = if depth < self.start_depth as usize {
                key[depth]
            } else {
                match self.peek(depth) {
                    Peek::Fragment(byte) => byte,
                    Peek::Branch(_) => panic!(),
                }
            }
        }

        Head::<KEY_LEN>::from(Self {
            _tag: HeadTag::Leaf,
            start_depth: actual_start_depth as u8,
            fragment: new_fragment,
        })
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct SharedKey<const KEY_LEN: usize> {
    key: [u8; KEY_LEN]
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct SharedLeaf<const KEY_LEN: usize> {
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; 6],
    body: Arc<SharedKey<KEY_LEN>>,
}

impl<const KEY_LEN: usize> From<SharedLeaf<KEY_LEN>> for Head<KEY_LEN> {
    fn from(head: SharedLeaf<KEY_LEN>) -> Self {
        unsafe { transmute(head) }
    }
}


impl<const KEY_LEN: usize> SharedLeaf<KEY_LEN> {
    pub(super) fn new(start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let mut fragment = [0; 6];

        fragment[..].copy_from_slice(&key[start_depth..start_depth+6]);

        Self {
            tag: HeadTag::SharedLeaf,
            start_depth: start_depth as u8,
            fragment,
            body: Arc::new(SharedKey {
                key: *key,
            }),
        }
    }
}

impl<const KEY_LEN: usize> HeadVariant<KEY_LEN> for SharedLeaf<KEY_LEN> {
    fn count(&self) -> u64 {
        1
    }

    fn peek(&self, at_depth: usize) -> Peek {
        assert!(
            self.start_depth as usize <= at_depth && at_depth < KEY_LEN as usize,
            "Peek out of bounds: {} <= {} < {}",
            self.start_depth,
            at_depth,
            KEY_LEN
        );
        match at_depth {
            depth if depth < self.start_depth as usize + self.fragment.len() => {
                Peek::Fragment(self.fragment[index_start(self.start_depth as usize, depth)])

            }
            depth => Peek::Fragment(self.body.key[depth]),
        }
    }

    fn get(&self, at_depth: usize, key: u8) -> Head<KEY_LEN> {
        match self.peek(at_depth) {
            Peek::Fragment(byte) if byte == key => self.clone().into(),
            _ => Empty::new().into(),
        }
    }

    fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        let mut depth = self.start_depth as usize;
        loop {
            if depth == KEY_LEN {
                return self.clone().into();
            }
            match self.peek(depth) {
                Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                Peek::Fragment(_) => {
                    let sibling_leaf = new_leaf(depth, key);

                    let mut new_branch = Branch4::new(self.start_depth as usize, depth, key);
                    new_branch.insert(key, sibling_leaf);
                    new_branch.insert(
                        key,
                        Head::<KEY_LEN>::from(self.clone()).wrap_path(depth, key),
                    );

                    return Head::<KEY_LEN>::from(new_branch)
                        .wrap_path(self.start_depth as usize, key);
                }
                Peek::Branch(_) => panic!(),
            }
        }
    }

    fn hash(&self, _prefix: &[u8; KEY_LEN]) -> u128 {
        let mut hasher = SipHasher24::new_with_key(unsafe { &SIP_KEY });
        hasher.write(&self.body.key[..]);
        return hasher.finish128().into();
    }

    fn with_start_depth(
        &self,
        new_start_depth: usize,
        _key: &[u8; KEY_LEN],
    ) -> Head<KEY_LEN> {
        let mut fragment = [0; 6];                
        fragment[..].copy_from_slice(&self.body.key[new_start_depth..new_start_depth+6]);

        Head::from(Self {
            tag: HeadTag::SharedLeaf,
            start_depth: new_start_depth as u8,
            fragment,
            body: Arc::clone(&self.body),
        })
    }
}

pub(super) fn new_leaf<const KEY_LEN: usize>(start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
    if KEY_LEN - start_depth < LEAF_FRAGMENT_LEN {
        Leaf::new(start_depth, key).into()
    } else {
        SharedLeaf::new(start_depth, key).into()
    }
}