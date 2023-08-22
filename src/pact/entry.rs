use super::*;

#[derive(Debug)]
#[repr(C)]
pub struct Entry<const KEY_LEN: usize> {
    ptr: *mut Leaf<KEY_LEN>,
    pub hash: u128,
}

impl<const KEY_LEN: usize> Entry<KEY_LEN> {
    pub fn new(key: &[u8; KEY_LEN]) -> Self {
        unsafe {
            let ptr = Leaf::<KEY_LEN>::new(key);
            let hash = Leaf::hash(ptr);

            Self { ptr, hash }
        }
    }

    pub(super) fn leaf<O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
        &self,
        start_depth: usize,
    ) -> Head<KEY_LEN, O, S> {
        unsafe {
            Head::new(
                HeadTag::Leaf,
                Leaf::peek::<O>(self.ptr, start_depth),
                Leaf::rc_inc(self.ptr),
            )
        }
    }

    pub(super) fn peek<O: KeyOrdering<KEY_LEN>>(&self, at_depth: usize) -> u8 {
        unsafe { Leaf::peek::<O>(self.ptr, at_depth) }
    }
}

impl<const KEY_LEN: usize> Clone for Entry<KEY_LEN> {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                ptr: Leaf::rc_inc(self.ptr),
                hash: self.hash,
            }
        }
    }
}

impl<const KEY_LEN: usize> Drop for Entry<KEY_LEN> {
    fn drop(&mut self) {
        unsafe {
            Leaf::rc_dec(self.ptr);
        }
    }
}
