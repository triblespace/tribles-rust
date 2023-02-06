use super::*;

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct InlineLeaf<const KEY_LEN: usize> {
    _tag: HeadTag,
    start_depth: u8,
    fragment: [u8; LEAF_FRAGMENT_LEN],
}

impl<const KEY_LEN: usize> From<InlineLeaf<KEY_LEN>> for Head<KEY_LEN> {
    fn from(head: InlineLeaf<KEY_LEN>) -> Self {
        unsafe { transmute(head) }
    }
}

impl<const KEY_LEN: usize> InlineLeaf<KEY_LEN> {
    pub(super) fn new(start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let mut fragment = [0; LEAF_FRAGMENT_LEN];

        copy_start(fragment.as_mut_slice(), &key[..], start_depth);

        Self {
            _tag: HeadTag::Leaf,
            start_depth: start_depth as u8,
            fragment,
        }
    }
}

impl<const KEY_LEN: usize> HeadVariant<KEY_LEN> for InlineLeaf<KEY_LEN> {
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
                    let sibling_leaf = new_leaf(depth, key);

                    let mut new_branch = Branch4::new(self.start_depth as usize, depth, key);
                    new_branch.insert(key, sibling_leaf);
                    new_branch.insert(
                        key,
                        self.clone().with_start(depth, key)
                    );

                    return Head::<KEY_LEN>::from(new_branch);
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

    fn with_start(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        assert!(new_start_depth < KEY_LEN);

        if KEY_LEN - new_start_depth < LEAF_FRAGMENT_LEN {
            let mut new_fragment = [0; LEAF_FRAGMENT_LEN];
            for depth in new_start_depth..KEY_LEN {
                if depth == KEY_LEN {
                    break;
                }
    
                new_fragment[depth - new_start_depth] = if depth < self.start_depth as usize {
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
                start_depth: new_start_depth as u8,
                fragment: new_fragment,
            })
        } else {
            let mut old_key = *key;
            old_key[self.start_depth as usize..].copy_from_slice(&self.fragment[KEY_LEN - self.start_depth as usize..]);

            return Leaf::new(new_start_depth, &old_key).into()
        }
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct LeafBody<const KEY_LEN: usize> {
    key: [u8; KEY_LEN]
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct Leaf<const KEY_LEN: usize> {
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; 6],
    key: Arc<LeafBody<KEY_LEN>>,
}

impl<const KEY_LEN: usize> From<Leaf<KEY_LEN>> for Head<KEY_LEN> {
    fn from(head: Leaf<KEY_LEN>) -> Self {
        unsafe { transmute(head) }
    }
}


impl<const KEY_LEN: usize> Leaf<KEY_LEN> {
    pub(super) fn new(start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let mut fragment = [0; 6];

        fragment[..].copy_from_slice(&key[start_depth..start_depth+6]);

        Self {
            tag: HeadTag::SharedLeaf,
            start_depth: start_depth as u8,
            fragment,
            key: Arc::new(LeafBody {
                key: *key,
            }),
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
        match at_depth {
            depth if depth < self.start_depth as usize + self.fragment.len() => {
                Peek::Fragment(self.fragment[index_start(self.start_depth as usize, depth)])

            }
            depth => Peek::Fragment(self.key.key[depth]),
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
                        self.clone().with_start(depth, key),
                    );

                    return Head::<KEY_LEN>::from(new_branch);
                }
                Peek::Branch(_) => panic!(),
            }
        }
    }

    fn hash(&self, _prefix: &[u8; KEY_LEN]) -> u128 {
        let mut hasher = SipHasher24::new_with_key(unsafe { &SIP_KEY });
        hasher.write(&self.key.key[..]);
        return hasher.finish128().into();
    }

    fn with_start(
        &self,
        new_start_depth: usize,
        _key: &[u8; KEY_LEN],
    ) -> Head<KEY_LEN> {
        let mut fragment = [0; 6];                
        fragment[..].copy_from_slice(&self.key.key[new_start_depth..new_start_depth+6]);

        Head::from(Self {
            tag: HeadTag::SharedLeaf,
            start_depth: new_start_depth as u8,
            fragment,
            key: Arc::clone(&self.key),
        })
    }
}

pub(super) fn new_leaf<const KEY_LEN: usize>(start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
    if KEY_LEN - start_depth < LEAF_FRAGMENT_LEN {
        InlineLeaf::new(start_depth, key).into()
    } else {
        Leaf::new(start_depth, key).into()
    }
}