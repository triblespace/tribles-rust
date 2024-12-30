use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use siphasher::sip128::{Hasher128, SipHasher24};
use std::alloc::*;
use std::ptr::addr_of;

use super::*;

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Leaf<const KEY_LEN: usize> {
    pub key: [u8; KEY_LEN],
    hash: u128,
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
            let mut hasher = SipHasher24::new_with_key(&*addr_of!(SIP_KEY));
            hasher.write(&key[..]);
            let hash = hasher.finish128().into();

            std::ptr::write(
                ptr,
                Self {
                    key: *key,
                    hash,
                    rc: atomic::AtomicU32::new(1),
                },
            );

            ptr
        }
    }

    pub(crate) unsafe fn rc_inc(leaf: *mut Self) -> *mut Self {
        unsafe {
            let mut current = (*leaf).rc.load(Relaxed);
            loop {
                if current == u32::MAX {
                    panic!("max refcount exceeded");
                }
                match (*leaf)
                    .rc
                    .compare_exchange(current, current + 1, Relaxed, Relaxed)
                {
                    Ok(_) => return leaf,
                    Err(v) => current = v,
                }
            }
        }
    }

    pub(crate) unsafe fn rc_dec(node: *mut Self) {
        unsafe {
            let rc = (*node).rc.fetch_sub(1, Release);
            if rc != 1 {
                return;
            }
            (*node).rc.load(Acquire);

            std::ptr::drop_in_place(node);

            let layout = Layout::new::<Self>();
            let ptr = node as *mut u8;
            dealloc(ptr, layout);
        }
    }

    pub(crate) unsafe fn hash(node: *const Self) -> u128 {
        (*node).hash
    }

    pub(crate) unsafe fn infixes<
        const PREFIX_LEN: usize,
        const INFIX_LEN: usize,
        O: KeyOrdering<KEY_LEN>,
        S: KeySegmentation<KEY_LEN>,
        F,
    >(
        leaf: *const Self,
        prefix: &[u8; PREFIX_LEN],
        at_depth: usize,
        f: &mut F,
    ) where
        F: FnMut(&[u8; INFIX_LEN]),
    {
        let leaf_key = &(*leaf).key;
        for depth in at_depth..PREFIX_LEN {
            if leaf_key[O::key_index(depth)] != prefix[depth] {
                return;
            }
        }

        let infix: [u8; INFIX_LEN] =
            core::array::from_fn(|i| (*leaf).key[O::key_index(PREFIX_LEN + i)]);
        f(&infix);
    }

    pub(crate) unsafe fn has_prefix<O: KeyOrdering<KEY_LEN>, const PREFIX_LEN: usize>(
        leaf: *const Self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        let leaf_key: &[u8; KEY_LEN] = &(*leaf).key;
        for depth in at_depth..PREFIX_LEN {
            if leaf_key[O::key_index(depth)] != prefix[depth] {
                return false;
            }
        }
        return true;
    }

    pub(crate) unsafe fn segmented_len<O: KeyOrdering<KEY_LEN>, const PREFIX_LEN: usize>(
        node: *const Self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        let leaf_key: &[u8; KEY_LEN] = &(*node).key;
        for depth in at_depth..PREFIX_LEN {
            let key_depth = O::key_index(depth);
            if leaf_key[key_depth] != prefix[depth] {
                return 0;
            }
        }
        return 1;
    }
}
