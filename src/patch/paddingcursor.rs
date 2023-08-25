/*
use super::*;

pub struct PaddedCursor<const KEY_LEN: usize, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
{
    inner: PATCHCursor<KEY_LEN, K>,
    depth: u8,
}

impl<const KEY_LEN: usize, K> PaddedCursor<KEY_LEN, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
{
    pub fn new(inner: PATCHCursor<KEY_LEN, K>) -> Self {
        Self { inner, depth: 0 }
    }
}

impl<const KEY_LEN: usize, K> ByteCursor for PaddedCursor<KEY_LEN, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
{
    const LEN: usize = KEY_LEN;

    fn peek(&self) -> Peek {
        if K::padding(self.depth as usize) {
            Peek::Fragment(0)
        } else {
            self.inner.peek()
        }
    }

    fn push(&mut self, byte: u8) {
        if !K::padding(self.depth as usize) {
            self.inner.push(byte);
        }
        self.depth += 1;
    }

    fn pop(&mut self) {
        self.depth -= 1;
        if !K::padding(self.depth as usize) {
            self.inner.pop();
        }
    }
}

impl<const KEY_LEN: usize, K> PaddedCursor<KEY_LEN, K>
where
    K: KeyProperties<KEY_LEN>,
    [Head<KEY_LEN, K>; KEY_LEN]: Sized,
{
    pub fn count_segment(&self) -> u32 {
        return self.inner.count_segment();
    }
}
*/
