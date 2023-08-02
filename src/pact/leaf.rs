use super::*;

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct Leaf<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; LEAF_FRAGMENT_LEN],
    key: SharedKey<KEY_LEN>,
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<S>,
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    From<Leaf<KEY_LEN, O, S>> for Head<KEY_LEN, O, S>
{
    fn from(head: Leaf<KEY_LEN, O, S>) -> Self {
        unsafe { transmute(head) }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Leaf<KEY_LEN, O, S>
{
    pub(super) fn new(start_depth: usize, key: &SharedKey<KEY_LEN>) -> Self {
        let mut fragment = [0; LEAF_FRAGMENT_LEN];

        copy_start(
            fragment.as_mut_slice(),
            &O::tree_ordered(key)[..],
            start_depth,
        );

        Self {
            tag: HeadTag::Leaf,
            start_depth: start_depth as u8,
            fragment,
            key: Arc::clone(key),
            key_ordering: PhantomData,
            key_segments: PhantomData,
        }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    HeadVariant<KEY_LEN, O, S> for Leaf<KEY_LEN, O, S>
{
    fn count(&self) -> u32 {
        1
    }

    fn count_segment(&self, _at_depth: usize) -> u32 {
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
            depth => Peek::Fragment(self.key[O::key_index(depth)]),
        }
    }

    fn child(&self, at_depth: usize, key: u8) -> Head<KEY_LEN, O, S> {
        match self.peek(at_depth) {
            Peek::Fragment(byte) if byte == key => self.clone().into(),
            _ => Empty::new().into(),
        }
    }

    fn hash(&self) -> u128 {
        let mut hasher = SipHasher24::new_with_key(unsafe { &SIP_KEY });
        hasher.write(&O::tree_ordered(&self.key)[..]);
        return hasher.finish128().into();
    }

    fn with_start(&self, new_start_depth: usize) -> Head<KEY_LEN, O, S> {
        let mut fragment = [0; LEAF_FRAGMENT_LEN];
        copy_start(
            fragment.as_mut_slice(),
            &O::tree_ordered(&self.key)[..],
            new_start_depth,
        );

        Head::from(Self {
            tag: HeadTag::Leaf,
            start_depth: new_start_depth as u8,
            fragment,
            key_ordering: PhantomData,
            key_segments: PhantomData,
            key: Arc::clone(&self.key),
        })
    }

    fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Head<KEY_LEN, O, S> {
        let mut depth = self.start_depth as usize;
        loop {
            if depth == KEY_LEN {
                return self.clone().into();
            }
            match self.peek(depth) {
                Peek::Fragment(byte) if byte == key[O::key_index(depth)] => depth += 1,
                Peek::Fragment(_) => {
                    let mut new_branch =
                        Branch4::new(self.start_depth as usize, depth, &O::tree_ordered(key));
                    new_branch.insert(Leaf::new(depth, key).into());
                    new_branch.insert(self.with_start(depth));

                    return Head::<KEY_LEN, O, S>::from(new_branch);
                }
                Peek::Branch(_) => panic!(),
            }
        }
    }

    fn infixes<const INFIX_LEN: usize, F>(
        &self,
        key: [u8; KEY_LEN],
        start_depth: usize,
        _end_depth: usize,
        f: F,
        out: &mut Vec<[u8; INFIX_LEN]>,
    ) where
        F: Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN],
    {
        let mut depth = self.start_depth as usize;
        loop {
            if start_depth <= depth {
                out.push(f(*self.key.as_ref()));
                return;
            }
            match self.peek(depth) {
                Peek::Fragment(byte) if byte == key[depth] => depth += 1,
                Peek::Fragment(_) => return,
                Peek::Branch(_) => panic!(),
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
                Peek::Branch(_) => panic!(),
            }
        }
    }

    fn segmented_len(&self, key: [u8; KEY_LEN], start_depth: usize) -> usize {
        1
    }
}
