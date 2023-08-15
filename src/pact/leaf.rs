use siphasher::sip128::{Hasher128, SipHasher24};
use std::alloc::*;
use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

use super::*;

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Leaf<const KEY_LEN: usize> {
    pub key: [u8; KEY_LEN],
    rc: atomic::AtomicU32,
}

impl<const KEY_LEN: usize> Leaf<KEY_LEN> {
    pub(super) unsafe fn new(key: &[u8; KEY_LEN]) -> *mut Self {
        unsafe {
            let layout = Layout::new::<Self>();
            let ptr = alloc(layout) as *mut Self;
            if ptr.is_null() {
                panic!("Allocation failed!");
            }
            std::ptr::write(ptr, Self {
                key: *key,
                rc: atomic::AtomicU32::new(1)
            });

            ptr
        }
    }

    pub(crate) unsafe fn rc_inc(node: *mut Self) -> *mut Self {
        unsafe {
            let mut current = (*node).rc.load(Relaxed);
            loop {
                if current == u32::MAX {
                    return Self::new(&(*node).key);
                }
                match (*node).rc.compare_exchange(current, current + 1, Relaxed, Relaxed) {
                    Ok(_) => return node,
                    Err(v) => current = v,
                }
            }
        }
    }

    pub(crate) unsafe fn rc_dec(node: *mut Self) {
        unsafe {
            if (*node).rc.fetch_sub(1, Release) != 1 {
                return;
            }
            (*node).rc.load(Acquire);

            let layout = Layout::new::<Self>();
            let ptr = node as *mut u8;
            dealloc(ptr, layout);
        }
    }

    pub(crate) unsafe fn peek<O: KeyOrdering<KEY_LEN>>(node: *const Self, at_depth: usize) -> u8 {
        unsafe { (*node).key[O::key_index(at_depth)] }
    }

    pub(crate) unsafe fn hash(node: *const Self) -> u128 {
        unsafe {
            let mut hasher = SipHasher24::new_with_key(&SIP_KEY);
            hasher.write(&(*node).key[..]);
            return hasher.finish128().into();
        }
    }

    pub(crate) unsafe fn with_start<O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
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

    pub(crate) unsafe fn put<O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
        head: &mut Head<KEY_LEN, O, S>,
        entry: &Entry<KEY_LEN>,
        at_depth: usize,
    ) {
        unsafe {
            let node: *const Self = head.ptr();
            for depth in at_depth..KEY_LEN {
                if Self::peek::<O>(node, depth) != entry.peek::<O>(depth) {
                    let new_branch = Branch4::new(depth);
                    Branch4::insert(new_branch, entry.leaf(depth), entry.hash);
                    Branch4::insert(new_branch, head.with_start(depth), head.hash());

                    Branch4::reset_min(new_branch);
                    
                    *head = Branch4::with_start(new_branch, at_depth);
                    return;
                }
            }
        }
    }

    pub(crate) unsafe fn infixes<
        const INFIX_LEN: usize,
        O: KeyOrdering<KEY_LEN>,
        S: KeySegmentation<KEY_LEN>,
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

    pub(crate) unsafe fn has_prefix<O: KeyOrdering<KEY_LEN>>(
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

    pub(crate) unsafe fn segmented_len<O: KeyOrdering<KEY_LEN>>(
        node: *mut Self,
        at_depth: usize,
        key: [u8; KEY_LEN],
        start_depth: usize,
    ) -> usize {
        for depth in at_depth..start_depth {
            if Leaf::peek::<O>(node, depth) != key[depth] {
                return 0;
            }
        }
        return 1;
    }
}
