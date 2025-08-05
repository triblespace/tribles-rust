use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use siphasher::sip128::SipHasher24;
use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::ptr::addr_of;

use super::*;

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Leaf<const KEY_LEN: usize> {
    pub key: [u8; KEY_LEN],
    pub hash: u128,
    rc: atomic::AtomicU32,
}

impl<const KEY_LEN: usize> Body for Leaf<KEY_LEN> {
    fn tag(_body: NonNull<Self>) -> HeadTag {
        HeadTag::Leaf
    }
}

impl<const KEY_LEN: usize> Leaf<KEY_LEN> {
    pub(super) unsafe fn new(key: &[u8; KEY_LEN]) -> NonNull<Self> {
        unsafe {
            let layout = Layout::new::<Self>();
            let Some(ptr) = NonNull::new(alloc(layout) as *mut Self) else {
                handle_alloc_error(layout);
            };
            let hash = SipHasher24::new_with_key(&*addr_of!(SIP_KEY))
                .hash(&key[..])
                .into();

            ptr.write(Self {
                key: *key,
                hash,
                rc: atomic::AtomicU32::new(1),
            });

            ptr
        }
    }

    pub(crate) unsafe fn rc_inc(leaf: NonNull<Self>) -> NonNull<Self> {
        unsafe {
            let leaf = leaf.as_ptr();
            let mut current = (*leaf).rc.load(Relaxed);
            loop {
                if current == u32::MAX {
                    panic!("max refcount exceeded");
                }
                match (*leaf)
                    .rc
                    .compare_exchange(current, current + 1, Relaxed, Relaxed)
                {
                    Ok(_) => return NonNull::new_unchecked(leaf),
                    Err(v) => current = v,
                }
            }
        }
    }

    pub(crate) unsafe fn rc_dec(leaf: NonNull<Self>) {
        unsafe {
            let ptr = leaf.as_ptr();
            let rc = (*ptr).rc.fetch_sub(1, Release);
            if rc != 1 {
                return;
            }
            (*ptr).rc.load(Acquire);

            std::ptr::drop_in_place(ptr);

            let layout = Layout::new::<Self>();
            let ptr = ptr as *mut u8;
            dealloc(ptr, layout);
        }
    }

    pub(crate) fn infixes<
        const PREFIX_LEN: usize,
        const INFIX_LEN: usize,
        O: KeyOrdering<KEY_LEN>,
        F,
    >(
        leaf: NonNull<Self>,
        prefix: &[u8; PREFIX_LEN],
        at_depth: usize,
        f: &mut F,
    ) where
        F: FnMut(&[u8; INFIX_LEN]),
    {
        let leaf = unsafe { leaf.as_ref() };
        let leaf_key = (*leaf).key;
        for depth in at_depth..PREFIX_LEN {
            if leaf_key[O::TREE_TO_KEY[depth]] != prefix[depth] {
                return;
            }
        }

        let infix: [u8; INFIX_LEN] =
            core::array::from_fn(|i| (*leaf).key[O::TREE_TO_KEY[PREFIX_LEN + i]]);
        f(&infix);
    }

    pub(crate) fn has_prefix<O: KeyOrdering<KEY_LEN>, const PREFIX_LEN: usize>(
        leaf: NonNull<Self>,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        const {
            assert!(PREFIX_LEN <= KEY_LEN);
        }
        let leaf_key: &[u8; KEY_LEN] = unsafe { &(*leaf.as_ptr()).key };
        for depth in at_depth..PREFIX_LEN {
            if leaf_key[O::TREE_TO_KEY[depth]] != prefix[depth] {
                return false;
            }
        }
        return true;
    }

    pub(crate) fn segmented_len<O: KeyOrdering<KEY_LEN>, const PREFIX_LEN: usize>(
        leaf: NonNull<Self>,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        let leaf_key: &[u8; KEY_LEN] = unsafe { &(*leaf.as_ptr()).key };
        for depth in at_depth..PREFIX_LEN {
            let key_depth = O::TREE_TO_KEY[depth];
            if leaf_key[key_depth] != prefix[depth] {
                return 0;
            }
        }
        return 1;
    }
}
