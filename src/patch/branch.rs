use super::*;
use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr::addr_of_mut;

const BRANCH_ALIGN: usize = 16;
const BRANCH_BASE_SIZE: usize = 48;
const TABLE_ENTRY_SIZE: usize = 8;

#[derive(Debug)]
#[repr(C, align(16))]
pub(crate) struct Branch<
    const KEY_LEN: usize,
    O: KeyOrdering<KEY_LEN>,
    S: KeySegmentation<KEY_LEN>,
    Table: ?Sized,
> {
    key_ordering: PhantomData<O>,
    key_segments: PhantomData<S>,

    rc: atomic::AtomicU32,
    pub end_depth: u32,
    pub childleaf: *const Leaf<KEY_LEN>,
    pub leaf_count: u64,
    pub segment_count: u64,
    pub hash: u128,
    pub child_table: Table,
}

impl<
        const BRANCHING_FACTOR: usize,
        const KEY_LEN: usize,
        O: KeyOrdering<KEY_LEN>,
        S: KeySegmentation<KEY_LEN>,
    > Body for Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>; BRANCHING_FACTOR]>
{
    fn tag(_body: NonNull<Self>) -> HeadTag {
        unsafe { transmute(BRANCHING_FACTOR as u8) }
    }
}

pub(crate) type BranchN<const KEY_LEN: usize, O, S> =
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>;

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>> Body
    for Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>
{
    fn tag(body: NonNull<Self>) -> HeadTag {
        unsafe { transmute((*body.as_ptr()).child_table.len().ilog2() as u8) }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>
{
    pub(super) fn new(
        head_key: u8,
        end_depth: usize,
        child: Head<KEY_LEN, O, S>,
    ) -> Head<KEY_LEN, O, S> {
        unsafe {
            let size = 2;
            let layout =
                Layout::from_size_align(BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * size), BRANCH_ALIGN)
                    .unwrap(); // TODO use unchecked if this doesn't fail immedaitately
            if let Some(ptr) =
                NonNull::new(std::ptr::slice_from_raw_parts(alloc_zeroed(layout), size)
                    as *mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>)
            {
                addr_of_mut!((*ptr.as_ptr()).rc).write(atomic::AtomicU32::new(1));
                addr_of_mut!((*ptr.as_ptr()).end_depth).write(end_depth as u32);
                addr_of_mut!((*ptr.as_ptr()).childleaf).write(child.childleaf());
                addr_of_mut!((*ptr.as_ptr()).leaf_count).write(child.count());
                addr_of_mut!((*ptr.as_ptr()).segment_count).write(child.count_segment(end_depth));
                addr_of_mut!((*ptr.as_ptr()).hash).write(child.hash());
                (*ptr.as_ptr()).child_table[0] = Some(child);

                Head::new(head_key, ptr)
            } else {
                panic!("Allocation failed!");
            }
        }
    }

    pub(super) fn new2(
        head_key: u8,
        end_depth: usize,
        lchild: Head<KEY_LEN, O, S>,
        rchild: Head<KEY_LEN, O, S>,
    ) -> Head<KEY_LEN, O, S> {
        unsafe {
            let size = 2;
            let layout =
                Layout::from_size_align(BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * size), BRANCH_ALIGN)
                    .unwrap(); // TODO use unchecked if this doesn't fail immedaitately
            if let Some(ptr) =
                NonNull::new(std::ptr::slice_from_raw_parts(alloc_zeroed(layout), size)
                    as *mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>)
            {
                addr_of_mut!((*ptr.as_ptr()).rc).write(atomic::AtomicU32::new(1));
                addr_of_mut!((*ptr.as_ptr()).end_depth).write(end_depth as u32);
                addr_of_mut!((*ptr.as_ptr()).childleaf).write(lchild.childleaf());
                addr_of_mut!((*ptr.as_ptr()).leaf_count).write(lchild.count() + rchild.count());
                addr_of_mut!((*ptr.as_ptr()).segment_count)
                    .write(lchild.count_segment(end_depth) + rchild.count_segment(end_depth));
                addr_of_mut!((*ptr.as_ptr()).hash).write(lchild.hash() ^ rchild.hash());
                (*ptr.as_ptr()).child_table[0] = Some(lchild);
                (*ptr.as_ptr()).child_table[1] = Some(rchild);

                Head::new(head_key, ptr)
            } else {
                panic!("Allocation failed!");
            }
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

            let size = (*branch).child_table.len();

            std::ptr::drop_in_place(branch);

            let layout =
                Layout::from_size_align(BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * size), BRANCH_ALIGN)
                    .unwrap(); // TODO use unchecked if this doesn't fail immedaitately
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
                let size = (*branch).child_table.len();
                let layout = Layout::from_size_align(
                    BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * size),
                    BRANCH_ALIGN,
                )
                .unwrap(); // TODO use unchecked if this doesn't fail immedaitately
                if let Some(ptr) =
                    NonNull::new(std::ptr::slice_from_raw_parts(alloc_zeroed(layout), size)
                        as *mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>)
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
                    panic!("Allocation failed!");
                }
            }
        }
    }

    pub(crate) fn grow(branch: NonNull<Self>) -> NonNull<Self> {
        unsafe {
            let branch = branch.as_ptr();
            let old_size = (*branch).child_table.len();
            let new_size = old_size * 2;
            assert!(new_size <= 256);

            let layout = Layout::from_size_align(
                BRANCH_BASE_SIZE + (TABLE_ENTRY_SIZE * new_size),
                BRANCH_ALIGN,
            )
            .unwrap(); // TODO use unchecked if this doesn't fail immedaitately
            if let Some(ptr) = NonNull::new(std::ptr::slice_from_raw_parts(
                alloc_zeroed(layout),
                new_size,
            )
                as *mut Branch<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>)
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

                Branch::<KEY_LEN, O, S, [Option<Head<KEY_LEN, O, S>>]>::rc_dec(
                    NonNull::new_unchecked(branch),
                );

                ptr
            } else {
                panic!("Allocation failed!");
            }
        }
    }

    pub fn insert_child(branch: NonNull<Self>, child: Head<KEY_LEN, O, S>) -> NonNull<Self> {
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

    pub fn count_segment(&self, at_depth: usize) -> u64 {
        if S::segment(O::key_index(at_depth)) != S::segment(O::key_index(self.end_depth as usize)) {
            1
        } else {
            self.segment_count
        }
    }

    pub fn childleaf_key(&self) -> &[u8; KEY_LEN] {
        unsafe { &(*self.childleaf).key }
    }

    pub(super) fn infixes<const PREFIX_LEN: usize, const INFIX_LEN: usize, F>(
        &self,
        prefix: &[u8; PREFIX_LEN],
        at_depth: usize,
        f: &mut F,
    ) where
        F: FnMut(&[u8; INFIX_LEN]),
    {
        let node_end_depth = self.end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = self.childleaf_key();
        for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
            if leaf_key[O::key_index(depth)] != prefix[depth] {
                return;
            }
        }

        // The infix ends within the current node.
        if PREFIX_LEN + INFIX_LEN <= node_end_depth {
            let infix: [u8; INFIX_LEN] =
                core::array::from_fn(|i| self.childleaf_key()[O::key_index(PREFIX_LEN + i)]);
            f(&infix);
            return;
        }
        // The prefix ends in a child of this node.
        if PREFIX_LEN > node_end_depth {
            if let Some(child) = self.child_table.table_get(prefix[node_end_depth]) {
                child.infixes(prefix, node_end_depth, f);
            }
            return;
        }

        // The prefix ends in this node, but the infix ends in a child.
        for entry in &self.child_table {
            if let Some(entry) = entry {
                entry.infixes(prefix, node_end_depth, f);
            }
        }
    }

    pub(super) fn has_prefix<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> bool {
        let node_end_depth = self.end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = self.childleaf_key();
        for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
            if leaf_key[O::key_index(depth)] != prefix[depth] {
                return false;
            }
        }

        // The prefix ends in this node.
        if PREFIX_LEN <= node_end_depth {
            return true;
        }

        //The prefix ends in a child of this node.
        if let Some(child) = self.child_table.table_get(prefix[node_end_depth]) {
            return child.has_prefix(node_end_depth, prefix);
        }
        // This node doesn't have a child matching the prefix.
        return false;
    }

    pub(super) fn segmented_len<const PREFIX_LEN: usize>(
        &self,
        at_depth: usize,
        prefix: &[u8; PREFIX_LEN],
    ) -> u64 {
        let node_end_depth = self.end_depth as usize;
        let leaf_key: &[u8; KEY_LEN] = self.childleaf_key();
        for depth in at_depth..std::cmp::min(node_end_depth, PREFIX_LEN) {
            let key_depth = O::key_index(depth);
            if leaf_key[key_depth] != prefix[depth] {
                return 0;
            }
        }
        if PREFIX_LEN <= node_end_depth {
            if S::segment(O::key_index(PREFIX_LEN)) != S::segment(O::key_index(node_end_depth)) {
                return 1;
            } else {
                return self.segment_count;
            }
        }
        if let Some(child) = self.child_table.table_get(prefix[node_end_depth]) {
            return child.segmented_len(node_end_depth, prefix);
        }
        return 0;
    }
}
