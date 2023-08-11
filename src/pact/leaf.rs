use std::alloc::*;

use super::*;

#[derive(Debug)]
#[repr(C)]
pub(super) struct Entry<const KEY_LEN: usize> {
    ptr: *mut Leaf<KEY_LEN>,
}

impl<const KEY_LEN: usize> Entry<KEY_LEN> {
    pub(super) fn new(key: &[u8; KEY_LEN]) -> Self {
        unsafe {
            let ptr = Leaf::<KEY_LEN>::new(key);

            Self { ptr }
        }
    }

    pub(super) fn leaf<O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
        &self,
        start_depth: usize,
    ) -> Head<KEY_LEN, O, S> {
        unsafe {
            Head::new(
                HeadTag::Leaf,
                (*self.ptr).key[O::key_index(start_depth)],
                self.ptr,
            )
        }
    }

    pub(super) fn peek<O: KeyOrdering<KEY_LEN>>(&self, at_depth: usize) -> u8 {
        unsafe { Leaf::peek::<O>(self.ptr, at_depth) }
    }
}

impl<const KEY_LEN: usize> Clone for Entry<KEY_LEN> {
    fn clone(&self) -> Self {
        Self {
            ptr: Leaf::rc_inc(self.ptr),
        }
    }
}

impl<const KEY_LEN: usize> Drop for Entry<KEY_LEN> {
    fn drop(&mut self) {
        Leaf::rc_dec(self.ptr);
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct Leaf<const KEY_LEN: usize> {
    pub key: [u8; KEY_LEN],
    rc: u32,
}

impl<const KEY_LEN: usize> Leaf<KEY_LEN> {
    pub(super) fn new(key: &[u8; KEY_LEN]) -> *mut Self {
        unsafe {
            let layout = Layout::new::<Self>();
            let ptr = alloc(layout) as *mut Self;
            if ptr.is_null() {
                panic!("Allocation failed!");
            }
            *ptr = Self { key: *key, rc: 1 };

            ptr
        }
    }

    pub(super) fn rc_inc(node: *mut Self) -> *mut Self {
        //TODO copy on overflow
        unsafe {
            (*node).rc = (*node).rc + 1;
            node
        }
    }

    pub(super) fn rc_dec(node: *mut Self) {
        unsafe {
            if (*node).rc == 1 {
                let layout = Layout::new::<Self>();
                let ptr = node as *mut u8;
                dealloc(ptr, layout);
            } else {
                (*node).rc = (*node).rc - 1
            }
        }
    }

    pub fn peek<O: KeyOrdering<KEY_LEN>>(node: *const Self, at_depth: usize) -> u8 {
        unsafe { (*node).key[O::key_index(at_depth)] }
    }

    pub fn hash<O: KeyOrdering<KEY_LEN>>(&self) -> u128 {
        let mut hasher = SipHasher24::new_with_key(unsafe { &SIP_KEY });
        hasher.write(&O::tree_ordered(&self.key)[..]);
        return hasher.finish128().into();
    }

    pub fn with_start<O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
        node: *mut Self,
        new_start_depth: usize,
    ) -> Head<KEY_LEN, O, S> {
        unsafe {
            Head::new(
                HeadTag::Leaf,
                (*node).key[O::key_index(new_start_depth)],
                node,
            )
        }
    }

    pub fn put<O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
        node: *mut Self,
        entry: &Entry<KEY_LEN>,
        at_depth: usize,
    ) -> Head<KEY_LEN, O, S> {
        for depth in at_depth..KEY_LEN {
            if Self::peek::<O>(node, depth) != entry.peek::<O>(depth) {
                let new_branch = Branch4::new(depth);
                Branch4::insert(new_branch, entry.leaf(depth).into());
                Branch4::insert(new_branch, Self::with_start(node, depth));

                return Branch4::with_start(new_branch, at_depth);
            }
        }
        return Self::with_start(node, at_depth);
    }

    pub fn infixes<
        O: KeyOrdering<KEY_LEN>,
        S: KeySegmentation<KEY_LEN>,
        const INFIX_LEN: usize,
        F,
    >(
        node: *mut Self,
        key: [u8; KEY_LEN],
        at_depth: usize,
        start_depth: usize,
        f: F,
        out: &mut Vec<[u8; INFIX_LEN]>,
    ) where
        F: Fn([u8; KEY_LEN]) -> [u8; INFIX_LEN],
    {
        for depth in at_depth..start_depth {
            if Leaf::peek::<O>(node, depth) != key[depth] {
                return;
            }
        }
        unsafe {
            out.push(f((*node).key));
        }
    }

    pub fn has_prefix<O: KeyOrdering<KEY_LEN>>(
        node: *mut Self,
        at_depth: usize,
        key: [u8; KEY_LEN],
        end_depth: usize,
    ) -> bool {
        for depth in at_depth..=end_depth {
            if Leaf::peek::<O>(node, depth) != key[depth] {
                return false;
            }
        }
        return true;
    }
}
