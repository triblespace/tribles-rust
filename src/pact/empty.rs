use super::*;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub(super) struct Empty<const KEY_LEN: usize> {
    tag: HeadTag,
    ignore: [MaybeUninit<u8>; 15],
}

impl<const KEY_LEN: usize> From<Empty<KEY_LEN>> for Head<KEY_LEN> {
    fn from(head: Empty<KEY_LEN>) -> Self {
        unsafe { transmute(head) }
    }
}
impl<const KEY_LEN: usize> Empty<KEY_LEN> {
    pub(super) fn new() -> Self {
        Self {
            tag: HeadTag::Empty,
            ignore: [MaybeUninit::new(0); 15],
        }
    }
}

impl<const KEY_LEN: usize> HeadVariant<KEY_LEN> for Empty<KEY_LEN> {
    fn count(&self) -> u64 {
        0
    }

    fn peek(&self, _at_depth: usize) -> Peek {
        Peek::Branch(ByteBitset::new_empty())
    }

    fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        new_leaf(0, key)
    }

    fn get(&self, _at_depth: usize, _key: u8) -> Head<KEY_LEN> {
        return Empty::new().into();
    }

    fn hash(&self, _prefix: &[u8; KEY_LEN]) -> u128 {
        0
    }
}
