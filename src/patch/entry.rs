use super::*;

#[derive(Debug)]
#[repr(C)]
pub struct Entry<const KEY_LEN: usize, V: Clone> {
    ptr: *mut Leaf<KEY_LEN, V>,
    pub hash: u128,
}

impl<const KEY_LEN: usize, V: Clone> Entry<KEY_LEN, V> {
    pub fn new(key: &[u8; KEY_LEN], value: V) -> Self {
        unsafe {
            let ptr = Leaf::<KEY_LEN, V>::new(key, value);
            let hash = Leaf::hash(ptr);

            Self { ptr, hash }
        }
    }

    pub(super) fn leaf<O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
        &self,
        start_depth: usize,
    ) -> Head<KEY_LEN, O, S, V> {
        unsafe {
            Head::new(
                HeadTag::Leaf,
                Leaf::peek(self.ptr, O::key_index(start_depth)),
                Leaf::rc_inc(self.ptr),
            )
        }
    }

    pub(super) fn peek(&self, at_depth: usize) -> u8 {
        unsafe { Leaf::peek(self.ptr, at_depth) }
    }
}

impl<const KEY_LEN: usize, V: Clone> Clone for Entry<KEY_LEN, V> {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                ptr: Leaf::rc_inc(self.ptr),
                hash: self.hash
            }
        }
    }
}

impl<const KEY_LEN: usize, V: Clone> Drop for Entry<KEY_LEN, V> {
    fn drop(&mut self) {
        unsafe {
            Leaf::rc_dec(self.ptr);
        }
    }
}
