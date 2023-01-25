use super::*;

#[derive(Clone, Debug)]
pub(super) struct Leaf<const KEY_LEN: usize> {
    tag: HeadTag,
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
            tag: HeadTag::Leaf,
            start_depth: actual_start_depth as u8,
            fragment: fragment,
        }
    }

    pub(super) fn count(&self) -> u64 {
        1
    }

    pub(super) fn peek(&self, at_depth: usize) -> Option<u8> {
        if KEY_LEN <= at_depth {
            return None;
        }
        return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }

    pub(super) fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
        result_set.unset_all();
        if KEY_LEN <= at_depth {
            return;
        }
        result_set.set(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }

    pub(super) fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        let mut branch_depth = self.start_depth as usize;
        while Some(key[branch_depth]) == self.peek(branch_depth) {
            branch_depth += 1;
        }
        if branch_depth == KEY_LEN {
            return self.clone().into();
        } else {
            let sibling_leaf = Head::<KEY_LEN>::from(Leaf::new(branch_depth, key));

            let mut new_branch = Branch4::new(self.start_depth as usize, branch_depth, key);
            new_branch.insert(sibling_leaf);
            new_branch.insert(Head::<KEY_LEN>::from(self.clone()).wrap_path(branch_depth, key));

            return Head::<KEY_LEN>::from(new_branch).wrap_path(self.start_depth as usize, key);
        }
    }

    pub(super) fn with_start_depth(
        &self,
        new_start_depth: usize,
        key: &[u8; KEY_LEN],
    ) -> Head<KEY_LEN> {
        assert!(new_start_depth <= KEY_LEN);

        let actual_start_depth = max(
            new_start_depth as isize,
            KEY_LEN as isize - (LEAF_FRAGMENT_LEN as isize),
        ) as usize;

        let mut new_fragment = [0; LEAF_FRAGMENT_LEN];
        for i in 0..new_fragment.len() {
            let depth = actual_start_depth + i;
            if KEY_LEN <= depth {
                break;
            }
            new_fragment[i] = if depth < self.start_depth as usize {
                key[depth]
            } else {
                self.fragment[index_start(self.start_depth as usize, depth)]
            }
        }

        Head::<KEY_LEN>::from(Self {
            tag: HeadTag::Leaf,
            start_depth: actual_start_depth as u8,
            fragment: new_fragment,
        })
    }

    pub(super) fn insert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`insert` called on leaf");
    }

    pub(super) fn reinsert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`reinsert` called on leaf");
    }

    pub(super) fn grow(&self) -> Head<KEY_LEN> {
        panic!("`grow` called on leaf");
    }
}
