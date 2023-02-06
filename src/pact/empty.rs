use super::*;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub(super) struct Empty<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> {
    tag: HeadTag,
    ignore: [MaybeUninit<u8>; 15],
    key_properties: PhantomData<K>,
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> From<Empty<KEY_LEN, K>> for Head<KEY_LEN, K> {
    fn from(head: Empty<KEY_LEN, K>) -> Self {
        unsafe { transmute(head) }
    }
}
impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> Empty<KEY_LEN, K> {
    pub(super) fn new() -> Self {
        Self {
            tag: HeadTag::Empty,
            ignore: [MaybeUninit::new(0); 15],
            key_properties: PhantomData,
        }
    }
}

impl<const KEY_LEN: usize, K: KeyProperties<KEY_LEN>> HeadVariant<KEY_LEN, K> for Empty<KEY_LEN, K> {
    fn count(&self) -> u64 {
        0
    }

    fn peek(&self, _at_depth: usize) -> Peek {
        Peek::Branch(ByteBitset::new_empty())
    }

    fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Head<KEY_LEN, K> {
        new_leaf(0, key)
    }

    fn get(&self, _at_depth: usize, _key: u8) -> Head<KEY_LEN, K> {
        return Empty::new().into();
    }

    fn hash(&self, _prefix: &[u8; KEY_LEN]) -> u128 {
        0
    }
}
