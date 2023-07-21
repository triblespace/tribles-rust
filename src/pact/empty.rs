use super::*;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub(super) struct Empty<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> {
    tag: HeadTag,
    ignore: [MaybeUninit<u8>; 15],
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<S>
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> From<Empty<KEY_LEN, O, S>> for Head<KEY_LEN, O, S> {
    fn from(head: Empty<KEY_LEN, O, S>) -> Self {
        unsafe { transmute(head) }
    }
}
impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Empty<KEY_LEN, O, S> {
    pub(super) fn new() -> Self {
        Self {
            tag: HeadTag::Empty,
            ignore: [MaybeUninit::new(0); 15],
            key_ordering: PhantomData,
            key_segments: PhantomData
        }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> HeadVariant<KEY_LEN, O, S>
    for Empty<KEY_LEN, O, S>
{
    fn count(&self) -> u32 {
        0
    }

    fn count_segment(&self, _depth: usize) -> u32 {
        0
    }

    fn peek(&self, _at_depth: usize) -> Peek {
        Peek::Branch(ByteBitset::new_empty())
    }

    fn child(&self, _at_depth: usize, _key: u8) -> Head<KEY_LEN, O, S> {
        return Empty::new().into();
    }

    fn hash(&self) -> u128 {
        0
    }

    fn put(&mut self, key: &SharedKey<KEY_LEN>) -> Head<KEY_LEN, O, S> {
        Leaf::new(0, key).into()
    }
    
    fn infixes<F>(&self, key: [u8;KEY_LEN], start_depth: usize, end_depth: usize, f: F)
    where F: FnMut([u8; KEY_LEN]) {
        return
    }
}
