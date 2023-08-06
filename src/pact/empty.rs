use super::*;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub(super) struct Empty<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
{
    tag: HeadTag,
    ignore: [MaybeUninit<u8>; 7],
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<S>,
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    From<Empty<KEY_LEN, O, S>> for Head<KEY_LEN, O, S>
{
    fn from(head: Empty<KEY_LEN, O, S>) -> Self {
        unsafe { transmute(head) }
    }
}
impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Empty<KEY_LEN, O, S>
{
    pub(super) fn new() -> Self {
        Self {
            tag: HeadTag::Empty,
            ignore: [MaybeUninit::new(0); 7],
            key_ordering: PhantomData,
            key_segments: PhantomData,
        }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    HeadVariant<KEY_LEN, O, S> for Empty<KEY_LEN, O, S>
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

    fn put(&mut self, key: &SharedKey<KEY_LEN>, start_depth: usize) -> Head<KEY_LEN, O, S> {
        Leaf::new(0, key)
    }

    fn infixes<const INFIX_LEN: usize, F>(
        &self,
        _key: [u8; KEY_LEN],
        _depth: usize,
        _start_depth: usize,
        _end_depth: usize,
        _f: F,
        out: &mut Vec<[u8; INFIX_LEN]>,
    ) where
        F: Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN],
    {
        return;
    }

    fn has_prefix(&self, depth: usize, key: [u8; KEY_LEN], end_depth: usize) -> bool {
        false
    }

    fn segmented_len(&self, depth: usize, key: [u8; KEY_LEN], start_depth: usize) -> usize {
        0
    }
}
