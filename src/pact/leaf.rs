use super::*;

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct Leaf<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> {
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; LEAF_FRAGMENT_LEN],
    key: SharedKey<KEY_LEN>,
    key_properties: PhantomData<K>,
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> From<Leaf<KEY_LEN, K>> for Head<KEY_LEN, K> {
    fn from(head: Leaf<KEY_LEN, K>) -> Self {
        unsafe { transmute(head) }
    }
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> Leaf<KEY_LEN, K> {
    pub(super) fn new(start_depth: usize, key: &SharedKey<KEY_LEN>) -> Self {
        let mut fragment = [0; LEAF_FRAGMENT_LEN];

        copy_start(
            fragment.as_mut_slice(),
            &reordered::<KEY_LEN, K>(key)[..],
            start_depth,
        );

        Self {
            tag: HeadTag::Leaf,
            start_depth: start_depth as u8,
            fragment,
            key: Arc::clone(key),
            key_properties: PhantomData,
        }
    }
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> HeadVariant<KEY_LEN, K> for Leaf<KEY_LEN, K> {
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
            depth => Peek::Fragment(self.key[K::reorder(depth)]),
        }
    }

    fn get(&self, at_depth: usize, key: u8) -> Head<KEY_LEN, K> {
        match self.peek(at_depth) {
            Peek::Fragment(byte) if byte == key => self.clone().into(),
            _ => Empty::new().into(),
        }
    }

    fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Head<KEY_LEN, K> {
        let mut depth = self.start_depth as usize;
        loop {
            if depth == KEY_LEN {
                return self.clone().into();
            }
            match self.peek(depth) {
                Peek::Fragment(byte) if byte == key[K::reorder(depth)] => depth += 1,
                Peek::Fragment(_) => {
                    let mut new_branch = Branch4::new(
                        self.start_depth as usize,
                        depth,
                        &reordered::<KEY_LEN, K>(key),
                    );
                    new_branch.insert(Leaf::new(depth, key).into());
                    new_branch.insert(self.with_start(depth));

                    return Head::<KEY_LEN, K>::from(new_branch);
                }
                Peek::Branch(_) => panic!(),
            }
        }
    }

    fn hash(&self) -> u128 {
        let mut hasher = SipHasher24::new_with_key(unsafe { &SIP_KEY });
        hasher.write(&reordered::<KEY_LEN, K>(&self.key)[..]);
        return hasher.finish128().into();
    }

    fn with_start(&self, new_start_depth: usize) -> Head<KEY_LEN, K> {
        let mut fragment = [0; LEAF_FRAGMENT_LEN];
        copy_start(
            fragment.as_mut_slice(),
            &reordered::<KEY_LEN, K>(&self.key)[..],
            new_start_depth,
        );

        Head::from(Self {
            tag: HeadTag::Leaf,
            start_depth: new_start_depth as u8,
            fragment,
            key_properties: PhantomData,
            key: Arc::clone(&self.key),
        })
    }
}
