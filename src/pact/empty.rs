use super::*;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub(super) struct Empty {
    tag: HeadTag,
    ignore: [MaybeUninit<u8>; 15],
}

impl<const KEY_LEN: usize> From<Empty> for Head<KEY_LEN> {
    fn from(head: Empty) -> Self {
        unsafe { transmute(head) }
    }
}
impl Empty {
    pub(super) fn new() -> Self {
        Self {
            tag: HeadTag::Empty,
            ignore: MaybeUninit::uninit_array(),
        }
    }

    pub(super) fn count(&self) -> u64 {
        0
    }

    pub(super) fn with_start_depth<const KEY_LEN: usize>(
        &self,
        _new_start_depth: usize,
        _key: &[u8; KEY_LEN],
    ) -> Head<KEY_LEN> {
        panic!("`with_start_depth` called on empty");
    }

    pub(super) fn peek(&self, _at_depth: usize) -> Option<u8> {
        None
    }

    pub(super) fn propose(&self, _at_depth: usize, result_set: &mut ByteBitset) {
        result_set.unset_all();
    }

    pub(super) fn put<const KEY_LEN: usize>(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        Head::<KEY_LEN>::from(Leaf::new(0, key)).wrap_path(0, key)
    }

    pub(super) fn hash<const KEY_LEN: usize>(&self, prefix: &[u8; KEY_LEN]) -> u128 {
        0
    }

    pub(super) fn insert<const KEY_LEN: usize>(&mut self, _key: &[u8; KEY_LEN], _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`insert` called on empty");
    }

    pub(super) fn reinsert<const KEY_LEN: usize>(
        &mut self,
        _child: Head<KEY_LEN>,
    ) -> Head<KEY_LEN> {
        panic!("`reinsert` called on empty");
    }

    pub(super) fn grow<const KEY_LEN: usize>(&self) -> Head<KEY_LEN> {
        panic!("`grow` called on empty");
    }
}
