use super::*;

#[derive(Debug)]
#[repr(C)]
pub struct Entry<const KEY_LEN: usize> {
    ptr: *mut Leaf<KEY_LEN>,
}

impl<const KEY_LEN: usize> Entry<KEY_LEN> {
    pub fn new(key: &[u8; KEY_LEN]) -> Self {
        unsafe {
            let ptr = Leaf::<KEY_LEN>::new(key);
            Self { ptr }
        }
    }

    pub(super) fn leaf<O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
        &self,
    ) -> Head<KEY_LEN, O, S> {
        unsafe { Head::new(HeadTag::Leaf, 0, Leaf::rc_inc(self.ptr)) }
    }
}

impl<const KEY_LEN: usize> Clone for Entry<KEY_LEN> {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                ptr: Leaf::rc_inc(self.ptr),
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
