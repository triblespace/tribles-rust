use super::*;
use core::sync::atomic;
use core::sync::atomic::Ordering::Acquire;
use core::sync::atomic::Ordering::Relaxed;
use core::sync::atomic::Ordering::Release;
use std::alloc::alloc_zeroed;
use std::alloc::dealloc;
use std::alloc::handle_alloc_error;
use std::alloc::Layout;
use std::ptr::addr_of;
use std::ptr::addr_of_mut;

const BRANCH_ALIGN: usize = 16;
const BRANCH_BASE_SIZE: usize = 48;
const TABLE_ENTRY_SIZE: usize = 8;

#[inline]
fn dst_len<T>(ptr: *const [T]) -> usize {
    let ptr: *const [()] = ptr as _;
    // SAFETY: There is no aliasing as () is zero-sized
    let slice: &[()] = unsafe { &*ptr };
    slice.len()
}

#[derive(Debug)]
#[repr(C, align(16))]
pub(crate) struct Branch<const KEY_LEN: usize, O: KeySchema<KEY_LEN>, Table: ?Sized, V> {
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<O::Segmentation>,

    rc: atomic::AtomicU32,
    pub end_depth: u32,
    pub childleaf: *const Leaf<KEY_LEN, V>,
    pub leaf_count: u64,
    pub segment_count: u64,
    pub hash: u128,
    pub child_table: Table,
}

impl<const BRANCHING_FACTOR: usize, const KEY_LEN: usize, O: KeySchema<KEY_LEN>, V> Body
    for Branch<KEY_LEN, O, [Option<Head<KEY_LEN, O, V>>; BRANCHING_FACTOR], V>
{
    fn tag(_body: NonNull<Self>) -> HeadTag {
        unsafe { transmute(BRANCHING_FACTOR as u8) }
    }
}

impl<const KEY_LEN: usize, O: KeySchema<KEY_LEN>, V> Body
    for Branch<KEY_LEN, O, [Option<Head<KEY_LEN, O, V>>], V>
{
    fn tag(body: NonNull<Self>) -> HeadTag {
        unsafe {
            let ptr = addr_of!((*body.as_ptr()).child_table);
            transmute(dst_len(ptr).ilog2() as u8)
        }
    }
}

impl<const KEY_LEN: usize, O: KeySchema<KEY_LEN>, V>
    Branch<KEY_LEN, O, [Option<Head<KEY_LEN, O, V>>], V>
{
    pub(super) fn new(
        end_depth: usize,
        lchild: Head<KEY_LEN, O, V>,
        rchild: Head<KEY_LEN, O, V>,
    ) -> NonNull<Self> {
        unsafe {
            let size = 2;
            // SAFETY: `BRANCH_ALIGN` is a power of two and `size` is small enough
            // that the computed layout size is valid.
            let layout = Layout::from_size_align_unchecked(
                BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * size),
                BRANCH_ALIGN,
            );
            let Some(ptr) =
                NonNull::new(std::ptr::slice_from_raw_parts(alloc_zeroed(layout), size)
                    as *mut Branch<KEY_LEN, O, [Option<Head<KEY_LEN, O, V>>], V>)
            else {
                handle_alloc_error(layout);
            };
            addr_of_mut!((*ptr.as_ptr()).rc).write(atomic::AtomicU32::new(1));
            addr_of_mut!((*ptr.as_ptr()).end_depth).write(end_depth as u32);
            addr_of_mut!((*ptr.as_ptr()).childleaf).write(lchild.childleaf());
            addr_of_mut!((*ptr.as_ptr()).leaf_count).write(lchild.count() + rchild.count());
            addr_of_mut!((*ptr.as_ptr()).segment_count)
                .write(lchild.count_segment(end_depth) + rchild.count_segment(end_depth));
            addr_of_mut!((*ptr.as_ptr()).hash).write(lchild.hash() ^ rchild.hash());
            (*ptr.as_ptr()).child_table[0] = Some(lchild);
            (*ptr.as_ptr()).child_table[1] = Some(rchild);

            ptr
        }
    }

    pub(super) unsafe fn rc_inc(branch: NonNull<Self>) -> NonNull<Self> {
        unsafe {
            let branch = branch.as_ptr();
            let mut current = (*branch).rc.load(Relaxed);
            loop {
                if current == u32::MAX {
                    panic!("max refcount exceeded");
                }
                match (*branch)
                    .rc
                    .compare_exchange(current, current + 1, Relaxed, Relaxed)
                {
                    Ok(_) => return NonNull::new_unchecked(branch),
                    Err(v) => current = v,
                }
            }
        }
    }

    pub(super) unsafe fn rc_dec(branch: NonNull<Self>) {
        unsafe {
            let branch = branch.as_ptr();
            if (*branch).rc.fetch_sub(1, Release) != 1 {
                return;
            }
            (*branch).rc.load(Acquire);

            let size = dst_len(addr_of!((*branch).child_table));

            std::ptr::drop_in_place(branch);

            // SAFETY: layout parameters are constructed from constants and a
            // runtime `size` that ensures alignment and size validity.
            let layout = Layout::from_size_align_unchecked(
                BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * size),
                BRANCH_ALIGN,
            );
            let ptr = branch as *mut u8;
            dealloc(ptr, layout);
        }
    }

    pub(super) unsafe fn rc_cow(branch: NonNull<Self>) -> Option<NonNull<Self>> {
        unsafe {
            let branch = branch.as_ptr();
            if (*branch).rc.load(Acquire) == 1 {
                None
            } else {
                let size = dst_len(addr_of!((*branch).child_table));
                // SAFETY: `size` preserves alignment requirements and the size
                // calculation cannot overflow for the allowed range.
                let layout = Layout::from_size_align_unchecked(
                    BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * size),
                    BRANCH_ALIGN,
                );
                if let Some(ptr) =
                    NonNull::new(std::ptr::slice_from_raw_parts(alloc_zeroed(layout), size)
                        as *mut Branch<KEY_LEN, O, [Option<Head<KEY_LEN, O, V>>], V>)
                {
                    addr_of_mut!((*ptr.as_ptr()).rc).write(atomic::AtomicU32::new(1));
                    addr_of_mut!((*ptr.as_ptr()).end_depth).write((*branch).end_depth);
                    addr_of_mut!((*ptr.as_ptr()).childleaf).write((*branch).childleaf);
                    addr_of_mut!((*ptr.as_ptr()).leaf_count).write((*branch).leaf_count);
                    addr_of_mut!((*ptr.as_ptr()).segment_count).write((*branch).segment_count);
                    addr_of_mut!((*ptr.as_ptr()).hash).write((*branch).hash);
                    (*ptr.as_ptr())
                        .child_table
                        .clone_from_slice(&(*branch).child_table);

                    Self::rc_dec(NonNull::new_unchecked(branch));
                    Some(ptr)
                } else {
                    handle_alloc_error(layout);
                }
            }
        }
    }

    pub(crate) fn grow(branch: NonNull<Self>) -> NonNull<Self> {
        unsafe {
            let branch = branch.as_ptr();
            let old_size = dst_len(addr_of!((*branch).child_table));
            let new_size = old_size * 2;
            assert!(new_size <= 256);

            // SAFETY: `new_size` is bounded and alignment is constant, so the
            // resulting layout is valid for allocation.
            let layout = Layout::from_size_align_unchecked(
                BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * new_size),
                BRANCH_ALIGN,
            );
            if let Some(ptr) = NonNull::new(std::ptr::slice_from_raw_parts(
                alloc_zeroed(layout),
                new_size,
            )
                as *mut Branch<KEY_LEN, O, [Option<Head<KEY_LEN, O, V>>], V>)
            {
                addr_of_mut!((*ptr.as_ptr()).rc).write(atomic::AtomicU32::new(1));
                addr_of_mut!((*ptr.as_ptr()).end_depth).write((*branch).end_depth);
                addr_of_mut!((*ptr.as_ptr()).leaf_count).write((*branch).leaf_count);
                addr_of_mut!((*ptr.as_ptr()).segment_count).write((*branch).segment_count);
                addr_of_mut!((*ptr.as_ptr()).childleaf).write((*branch).childleaf);
                addr_of_mut!((*ptr.as_ptr()).hash).write((*branch).hash);
                // Note that the child_table is already zeroed by the allocator and therefore None initialized.

                (*branch)
                    .child_table
                    .table_grow(&mut (*ptr.as_ptr()).child_table);

                Branch::<KEY_LEN, O, [Option<Head<KEY_LEN, O, V>>], V>::rc_dec(
                    NonNull::new_unchecked(branch),
                );

                ptr
            } else {
                handle_alloc_error(layout);
            }
        }
    }

    pub fn insert_child(branch: NonNull<Self>, child: Head<KEY_LEN, O, V>) -> NonNull<Self> {
        unsafe {
            let mut branch = branch.as_ptr();
            let end_depth = (*branch).end_depth as usize;
            (*branch).leaf_count += child.count();
            (*branch).segment_count += child.count_segment(end_depth);
            (*branch).hash ^= child.hash();

            let mut displaced = child;
            loop {
                let Some(new_displaced) = (*branch).child_table.table_insert(displaced) else {
                    return NonNull::new_unchecked(branch);
                };
                displaced = new_displaced;
                branch = Self::grow(NonNull::new_unchecked(branch as _)).as_ptr();
            }
        }
    }

    #[must_use]
    pub fn upsert_child<F>(
        branch: NonNull<Self>,
        inserted: Head<KEY_LEN, O, V>,
        update: F,
    ) -> NonNull<Self>
    where
        F: FnOnce(&mut Option<Head<KEY_LEN, O, V>>, Head<KEY_LEN, O, V>),
    {
        unsafe {
            let ptr = branch.as_ptr();
            let inserted = inserted.with_start((*ptr).end_depth as usize);
            let key = inserted.key();
            if let Some(slot) = (*ptr).child_table.table_get_slot(key) {
                let child = slot.as_ref().unwrap();
                let old_child_hash = child.hash();
                let old_child_segment_count = child.count_segment((*ptr).end_depth as usize);
                let old_child_leaf_count = child.count();
                update(slot, inserted);

                let child = slot.as_ref().expect("upsert may not remove child");
                (*ptr).childleaf = child.childleaf();

                (*ptr).hash = ((*ptr).hash ^ old_child_hash) ^ child.hash();
                (*ptr).segment_count = ((*ptr).segment_count - old_child_segment_count)
                    + child.count_segment((*ptr).end_depth as usize);
                (*ptr).leaf_count = ((*ptr).leaf_count - old_child_leaf_count) + child.count();
                branch
            } else {
                branch::Branch::insert_child(branch, inserted)
            }
        }
    }

    pub fn update_child<F>(branch: NonNull<Self>, key: u8, update: F)
    where
        F: FnOnce(Head<KEY_LEN, O, V>) -> Option<Head<KEY_LEN, O, V>>,
    {
        unsafe {
            let ptr = branch.as_ptr();
            if let Some(slot) = (*ptr).child_table.table_get_slot(key) {
                let child = slot.take().unwrap();
                let old_child_hash = child.hash();
                let old_child_segment_count = child.count_segment((*ptr).end_depth as usize);
                let old_child_leaf_count = child.count();

                if let Some(new_child) = update(child) {
                    (*ptr).hash = ((*ptr).hash ^ old_child_hash) ^ new_child.hash();
                    (*ptr).segment_count = ((*ptr).segment_count - old_child_segment_count)
                        + new_child.count_segment((*ptr).end_depth as usize);
                    (*ptr).leaf_count =
                        ((*ptr).leaf_count - old_child_leaf_count) + new_child.count();

                    if slot.replace(new_child.with_key(key)).is_some() {
                        unreachable!();
                    }
                } else {
                    (*ptr).hash ^= old_child_hash;
                    (*ptr).segment_count -= old_child_segment_count;
                    (*ptr).leaf_count -= old_child_leaf_count;
                }
            }
        }
    }

    pub fn count_segment(branch: NonNull<Self>, at_depth: usize) -> u64 {
        unsafe {
            let branch = branch.as_ptr();
            if <O as KeySchema<KEY_LEN>>::Segmentation::SEGMENTS[O::TREE_TO_KEY[at_depth]]
                != <O as KeySchema<KEY_LEN>>::Segmentation::SEGMENTS
                    [O::TREE_TO_KEY[(*branch).end_depth as usize]]
            {
                1
            } else {
                (*branch).segment_count
            }
        }
    }

    pub(super) fn infixes<const PREFIX_LEN: usize, const INFIX_LEN: usize, F>(
        branch: NonNull<Self>,
        prefix: &[u8; PREFIX_LEN],
        at_depth: usize,
        f: &mut F,
    ) where
        F: FnMut(&[u8; INFIX_LEN]),
    {
        unsafe {
            let branch = branch.as_ptr();
            let node_end_depth = (*branch).end_depth as usize;
            let leaf_key: &[u8; KEY_LEN] = &(*(*branch).childleaf).key;
            for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
                if leaf_key[O::TREE_TO_KEY[depth]] != prefix[depth] {
                    return;
                }
            }

            // The infix ends within the current node.
            if PREFIX_LEN + INFIX_LEN <= node_end_depth {
                let infix: [u8; INFIX_LEN] = core::array::from_fn(|i| {
                    (*(*branch).childleaf).key[O::TREE_TO_KEY[PREFIX_LEN + i]]
                });
                f(&infix);
                return;
            }
            // The prefix ends in a child of this node.
            if PREFIX_LEN > node_end_depth {
                if let Some(child) = (*branch).child_table.table_get(prefix[node_end_depth]) {
                    child.infixes(prefix, node_end_depth, f);
                }
                return;
            }

            // The prefix ends in this node, but the infix ends in a child.
            for entry in &(*branch).child_table {
                if let Some(entry) = entry {
                    entry.infixes(prefix, node_end_depth, f);
                }
            }
        }
    }

    pub(super) fn has_prefix<const PREFIX_LEN: usize>(
        branch: NonNull<Self>,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        const {
            assert!(PREFIX_LEN <= KEY_LEN);
        }
        unsafe {
            let branch = branch.as_ptr();
            let node_end_depth = (*branch).end_depth as usize;
            let leaf_key: &[u8; KEY_LEN] = &(*(*branch).childleaf).key;
            for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
                if leaf_key.get_unchecked(O::TREE_TO_KEY[depth]) != prefix.get_unchecked(depth) {
                    return false;
                }
            }

            // The prefix ends in this node.
            if PREFIX_LEN <= node_end_depth {
                return true;
            }

            //The prefix ends in a child of this node.
            if let Some(child) = (*branch).child_table.table_get(prefix[node_end_depth]) {
                return child.has_prefix(node_end_depth, prefix);
            }
            // This node doesn't have a child matching the prefix.
            false
        }
    }

    pub(super) fn get<'a>(
        branch: NonNull<Self>,
        at_depth: usize,
        key: &[u8; KEY_LEN],
    ) -> Option<&'a V>
    where
        O: 'a,
    {
        unsafe {
            let branch = branch.as_ptr();
            let node_end_depth = (*branch).end_depth as usize;
            let leaf_key: &[u8; KEY_LEN] = &(*(*branch).childleaf).key;
            for depth in at_depth..std::cmp::min(node_end_depth, KEY_LEN) {
                let idx = O::TREE_TO_KEY[depth];
                if leaf_key[idx] != key[idx] {
                    return None;
                }
            }

            if node_end_depth >= KEY_LEN {
                return Some(&(*(*branch).childleaf).value);
            }

            if let Some(child) = (*branch).child_table.table_get(key[node_end_depth]) {
                return child.get(node_end_depth, key);
            }
            None
        }
    }

    pub(super) fn segmented_len<const PREFIX_LEN: usize>(
        branch: NonNull<Self>,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        unsafe {
            let branch = branch.as_ptr();
            let node_end_depth = (*branch).end_depth as usize;
            let leaf_key: &[u8; KEY_LEN] = &(*(*branch).childleaf).key;
            for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
                let key_depth = O::TREE_TO_KEY[depth];
                if leaf_key[key_depth] != prefix[depth] {
                    return 0;
                }
            }
            if PREFIX_LEN <= node_end_depth {
                if <O as KeySchema<KEY_LEN>>::Segmentation::SEGMENTS[O::TREE_TO_KEY[PREFIX_LEN]]
                    != <O as KeySchema<KEY_LEN>>::Segmentation::SEGMENTS
                        [O::TREE_TO_KEY[node_end_depth]]
                {
                    return 1;
                } else {
                    return (*branch).segment_count;
                }
            }
            if let Some(child) = (*branch).child_table.table_get(prefix[node_end_depth]) {
                return child.segmented_len(node_end_depth, prefix);
            }
            0
        }
    }
}
