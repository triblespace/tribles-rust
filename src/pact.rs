use crate::bitset::ByteBitset;
use crate::bytetable::*;
//use siphasher::sip128::{Hasher128, SipHasher24};
use std::alloc::{Allocator, Global, Layout};
use std::cmp::{max, min};
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU16, Ordering};

#[inline]
pub unsafe fn sizeless_transmute<A, B>(a: A) -> B {
    let b = ::core::ptr::read(&a as *const A as *const B);
    ::core::mem::forget(a);
    b
}

pub trait SizeLimited<const LIMIT: usize>: Sized {
    const UNUSED: usize = LIMIT - std::mem::size_of::<Self>();
}

impl<A: Sized, const LIMIT: usize> SizeLimited<LIMIT> for A {}

const HEAD_SIZE: usize = 16;
const HEAD_FRAGMENT_LEN: usize = 5;

fn index_start(infix_start: usize, index: usize) -> usize {
    index - infix_start
}

fn index_end(infix_len: usize, infix_end: usize, index: usize) -> usize {
    (index + infix_len) - infix_end
}

fn copy_end(target: &mut [u8], source: &[u8], end_index: usize) {
    let target_len = target.len();
    let used_len = min(end_index as usize, target_len);
    let target_range = &mut target[target_len - used_len..];
    let source_range = &source[end_index - used_len..end_index];
    target_range.copy_from_slice(source_range);
}

fn copy_start(target: &mut [u8], source: &[u8], start_index: usize) {
    let target_len = target.len();
    let source_len = source.len();
    let used_len = min(source_len - start_index as usize, target_len);
    let target_range = &mut target[0..used_len];
    let source_range = &source[start_index..start_index as usize + used_len];
    target_range.copy_from_slice(source_range);
}

#[derive(Copy, Clone)]
#[repr(C)]
struct EmptyHead {
    tag: HeadTag,
    padding: [u8; 15],
}

impl<const KEY_LEN: usize, Value> From<EmptyHead> for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: EmptyHead) -> Self {
        sizeless_transmute::<EmptyHead, Head<KEY_LEN, Value>>(head)
    }
}

impl<const KEY_LEN: usize, Value> From<Head<KEY_LEN, Value>> for EmptyHead
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: Head<KEY_LEN, Value>) -> Self {
        assert_eq!(head.tag, EmptyHead::TAG);
        sizeless_transmute::<Head<KEY_LEN, Value>, EmptyHead>(head)
    }
}

impl<const KEY_LEN: usize, Value> AsRef<Head<KEY_LEN, Value>> for EmptyHead
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn as_ref(&self) -> &Head<KEY_LEN, Value> {
        std::mem::transmute(self)
    }
}

impl EmptyHead {
    const TAG: HeadTag = HeadTag::Empty;

    fn new<const KEY_LEN: usize, Value>() -> Head<KEY_LEN, Value>
    where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
    {
        (Self {
            tag: Self::TAG,
            padding: mem::zeroed(),
        })
        .into()
    }

    fn put<const KEY_LEN: usize, Value>(
        self,
        start_depth: usize,
        key: &[u8; KEY_LEN],
        value: Value,
    ) -> Head<KEY_LEN, Value>
    where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
    {
        LeafHead::<KEY_LEN, Value>::new(start_depth, key, value).wrap_path(start_depth, key)
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
struct LeafHead<const KEY_LEN: usize, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; <Value as SizeLimited<13>>::UNUSED + 1],
    value: Value,
}

impl<const KEY_LEN: usize, Value> From<LeafHead<KEY_LEN, Value>> for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: LeafHead<KEY_LEN, Value>) -> Self {
        sizeless_transmute::<LeafHead<KEY_LEN, Value>, Head<KEY_LEN, Value>>(head)
    }
}

impl<const KEY_LEN: usize, Value> From<Head<KEY_LEN, Value>> for LeafHead<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: Head<KEY_LEN, Value>) -> Self {
        assert_eq!(head.tag, LeafHead::<KEY_LEN, Value>::TAG);
        sizeless_transmute::<Head<KEY_LEN, Value>, LeafHead<KEY_LEN, Value>>(head)
    }
}

impl<const KEY_LEN: usize, Value> AsRef<Head<KEY_LEN, Value>> for LeafHead<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn as_ref(&self) -> &Head<KEY_LEN, Value> {
        std::mem::transmute(self)
    }
}

impl<const KEY_LEN: usize, Value> LeafHead<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const TAG: HeadTag = HeadTag::Leaf;
    const FRAGMENT_LEN: usize = <Value as SizeLimited<13>>::UNUSED + 1;
    const FRAGMENT_RANGE: usize = KEY_LEN - Self::FRAGMENT_LEN;

    /// We want to copy the last bytes of the key into the leaf fragment.
    /// Note how the position of the fragment changes relative to the key when the
    /// start_depth is outside of the range that can be covered by the fragment.
    ///
    ///
    /// Case: start_depth < fragment_range                     ┌──────────┐
    ///    ┌───────────────────────────────────────────────────┤ fragment │
    ///    │                             key                   └──────────┤
    ///    └──────────────────────────────────────▲────────────▲──────────▲
    ///                               start_depth─┘            │  KEY_LEN─┘
    ///                                         fragment_range─┘
    ///
    ///
    /// Case: start_depth > fragment_range                          ┌──────────┐
    ///    ┌────────────────────────────────────────────────────────┤ fragment │
    ///    │                             key                        └─────┬────┘
    ///    └───────────────────────────────────────────────────▲────▲─────▲
    ///                                         fragment_range─┘    │     │
    ///                                                 start_depth─┘     │
    ///                                                           KEY_LEN─┘
    ///
    fn new(start_depth: usize, key: &[u8; KEY_LEN], value: Value) -> Head<KEY_LEN, Value> {
        let actual_start_depth = max(start_depth, Self::FRAGMENT_RANGE);

        let mut leaf_head = Self {
            tag: Self::TAG,
            start_depth: actual_start_depth as u8,
            fragment: unsafe { mem::zeroed() },
            value: value.clone(),
        };

        copy_start(
            leaf_head.fragment.as_mut_slice(),
            &key[..],
            actual_start_depth,
        );

        leaf_head.into()
    }

    fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        let actual_start_depth = max(
            start_depth as isize,
            KEY_LEN as isize - Self::FRAGMENT_LEN as isize,
        ) as usize;
        self.start_depth = actual_start_depth as u8;
        copy_start(self.fragment.as_mut_slice(), &key[..], actual_start_depth);

        self.into()
    }

    fn peek(&self, at_depth: usize) -> Option<u8> {
        if KEY_LEN <= at_depth {
            return None; //TODO: do we need this vs. assert?
        }
        return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }

    pub fn put(
        self,
        start_depth: usize,
        key: &[u8; KEY_LEN],
        value: Value,
    ) -> Head<KEY_LEN, Value> {
        let mut branch_depth = start_depth;
        while branch_depth < KEY_LEN {
            if Some(key[branch_depth]) == self.peek(branch_depth) {
                branch_depth += 1
            } else {
                break;
            }
        }
        if branch_depth == KEY_LEN {
            return self.into();
        }

        let sibling_leaf_node = LeafHead::new(branch_depth, key, value);

        let mut branch_head = BranchHead4::<KEY_LEN, Value>::new(start_depth, branch_depth, key);
        branch_head.insert(sibling_leaf_node);
        branch_head.insert(self.expand(branch_depth, key));

        return branch_head.wrap_path(start_depth, key);
    }
}

macro_rules! create_grow {
    ($old_body_name:ident,,) => {};
    ($old_body_name:ident, $grown_head_name:ident, $grown_body_name:ident) => {
        fn grow(self) -> Head<KEY_LEN, Value> {
            if Self::TAG == HeadTag::Branch256 {
                return self.into();
            }
            unsafe {
                let old_layout = Layout::new::<$old_body_name<KEY_LEN, Value>>();
                let new_layout = Layout::new::<$grown_body_name<KEY_LEN, Value>>();
                let branch_body = Global
                    .grow(self.body.cast::<u8>(), old_layout, new_layout)
                    .unwrap()
                    .cast::<$grown_body_name<KEY_LEN, Value>>();

                branch_body.as_mut().child_table.grow_repair();

                let new_head = $grown_head_name::<KEY_LEN, Value> {
                    tag: $grown_head_name::<KEY_LEN, Value>::TAG,
                    start_depth: self.start_depth,
                    fragment: self.fragment,
                    end_depth: self.end_depth,
                    body: branch_body,
                    phantom: PhantomData,
                };

                new_head.into()
            }
        }
    };
}

macro_rules! create_branch {
    ($head_name:ident, $body_name:ident, $tag:expr, $table:tt, $($grown_head_name:ident)?, $($grown_body_name:ident)?) => {
        #[repr(C)]
        struct $body_name<const KEY_LEN: usize, Value>
        where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            leaf_count: u64,
            rc: AtomicU16,
            segment_count: u32, //TODO: increase this to a u48
            node_hash: u128,
            child_set: ByteBitset,
            child_table: $table<Head<KEY_LEN, Value>>,
        }

        #[repr(C)]
        struct $head_name<const KEY_LEN: usize, Value>
        where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            tag: HeadTag,
            start_depth: u8,
            fragment: [u8; HEAD_FRAGMENT_LEN],
            end_depth: u8,
            body: NonNull<$body_name<KEY_LEN, Value>>,
            phantom: PhantomData<$body_name<KEY_LEN, Value>>,
        }

        impl<const KEY_LEN: usize, Value> From<$head_name<KEY_LEN, Value>> for Head<KEY_LEN, Value>
        where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            fn from(head: $head_name<KEY_LEN, Value>) -> Self {
                unsafe { sizeless_transmute::<$head_name<KEY_LEN, Value>, Head<KEY_LEN, Value>>(head) }
            }
        }

        impl<const KEY_LEN: usize, Value> From<Head<KEY_LEN, Value>> for $head_name<KEY_LEN, Value>
        where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            fn from(head: Head<KEY_LEN, Value>) -> Self {
                assert_eq!(head.tag, $head_name::<KEY_LEN, Value>::TAG);
                sizeless_transmute::<Head<KEY_LEN, Value>, $head_name<KEY_LEN, Value>>(head)
            }
        }

        impl<const KEY_LEN: usize, Value>  AsRef<Head<KEY_LEN, Value>> for $head_name<KEY_LEN, Value>
        where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            fn as_ref(&self) -> &Head<KEY_LEN, Value> {
                std::mem::transmute(self)
            }
        }

        impl<const KEY_LEN: usize, Value> Clone for $head_name<KEY_LEN, Value>
        where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            fn clone(&self) -> Self {
                Self {
                    tag: Self::TAG,
                    start_depth: self.start_depth,
                    fragment: self.fragment,
                    end_depth: self.end_depth,
                    body: self.body,
                    phantom: PhantomData,
                }
            }
        }

        impl<const KEY_LEN: usize, Value> $head_name<KEY_LEN, Value>
        where
        Value: SizeLimited<13> + Clone,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            const TAG: HeadTag = $tag;
            const FRAGMENT_LEN: usize = HEAD_FRAGMENT_LEN;

            fn new(
                start_depth: usize,
                branch_depth: usize,
                key: &[u8; KEY_LEN],
            ) -> Head<KEY_LEN, Value> {
                unsafe {
                    let layout = Layout::new::<$body_name<KEY_LEN, Value>>();
                    let branch_body = Global
                    .allocate_zeroed(layout)
                    .unwrap()
                    .cast::<$body_name<KEY_LEN, Value>>();
                    *(branch_body.as_mut()) = $body_name {
                        leaf_count: 0,
                        rc: AtomicU16::new(1),
                        segment_count: 0,
                        node_hash: 0,
                        child_set: ByteBitset::new_empty(),
                        child_table: $table::new(),
                    };

                    let actual_start_depth = max(
                        start_depth as isize,
                        branch_depth as isize - Self::FRAGMENT_LEN as isize,
                    ) as usize;

                    let mut branch_head = Self {
                        tag: Self::TAG,
                        start_depth: actual_start_depth as u8,
                        fragment: mem::zeroed(),
                        end_depth: branch_depth as u8,
                        body: branch_body,
                        phantom: PhantomData,
                    };

                    copy_start(
                        branch_head.fragment.as_mut_slice(),
                        &key[..],
                        actual_start_depth,
                    );

                    branch_head.into()
                }
            }

            fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
                let actual_start_depth = max(
                    start_depth as isize,
                    self.end_depth as isize - Self::FRAGMENT_LEN as isize,
                ) as usize;
                self.start_depth = actual_start_depth as u8;
                copy_start(self.fragment.as_mut_slice(), &key[..], actual_start_depth);

                self.into()
            }

            fn insert(&mut self, child: Head<KEY_LEN, Value>) -> Head<KEY_LEN, Value> {
                self.body.as_mut().child_table.put(child)
            }

            fn put(
                self,
                start_depth: usize,
                key: &[u8; KEY_LEN],
                value: Value,
                subtree_clone: bool,
            ) -> Head<KEY_LEN, Value> {
                let body = unsafe {self.body.as_ref()};
                let needs_clone = subtree_clone || body.rc.load(Ordering::SeqCst) > 1;

                let mut branch_depth = start_depth;
                while branch_depth < self.end_depth as usize {
                    if Some(key[branch_depth]) == self.peek(branch_depth) {
                        branch_depth += 1
                    } else {
                        break;
                    }
                }
                if branch_depth == self.end_depth as usize {
                    // The entire compressed infix above this node matched with the key.
                    let byte_key = key[branch_depth];
                    if body.child_set.is_set(byte_key) {
                        // The node already has a child branch with the same byte byte_key as the one in the key.
                        let old_child = body.child_table.take(byte_key).unwrap();
                        //let old_child_hash = old_child.hash(key);
                        //let old_child_leaf_count = old_child.count();
                        //let old_child_segment_count = old_child.segmentCount(branch_depth);
                        let new_child = old_child.put(branch_depth, key, value, needs_clone);
                        //let new_child_hash = new_child.hash(key);

                        //let new_hash = self.body.node_hash.update(old_child_hash, new_child_hash);
                        //let new_leaf_count = self.body.leaf_count - old_child_leaf_count + new_child.count();
                        //let new_segment_count = self.body.segment_count - old_child_segment_count + new_child.segmentCount(branch_depth);

                        let mut cow = if needs_clone { self.clone() } else { self };
                        //cow.body.node_hash = new_hash;
                        //cow.body.leaf_count = new_leaf_count;
                        //cow.body.segment_count = new_segment_count;

                        cow.insert(new_child);
                        return cow.into();
                    }
                    let new_child =
                    LeafHead::new(branch_depth, key, value).wrap_path(branch_depth, key);

                    let mut cow = if needs_clone { self.clone() } else { self };

                    let displaced = cow.insert(new_child);
                    let grown: Head<KEY_LEN, Value> = cow.into();
                    while displaced.key().is_some() {
                        grown = grown.grow();
                        displaced = grown.insert(displaced);
                    }
                    return grown;
                }

                let sibling_leaf_node =
                LeafHead::new(branch_depth, key, value).wrap_path(branch_depth, key);

                let mut branch_head =
                BranchHead4::<KEY_LEN, Value>::new(start_depth, branch_depth, key);
                branch_head.insert(sibling_leaf_node);
                branch_head.insert(<Self as Into<Head<KEY_LEN, Value>>>::into(self).wrap_path(branch_depth, key));

                return branch_head.wrap_path(start_depth, key);
            }

            fn peek(self, at_depth: usize) -> Option<u8> {
                if at_depth < self.start_depth as usize || self.end_depth as usize <= at_depth {
                    return None;
                }
                return Some(
                    self.fragment[index_start(self.start_depth as usize, at_depth as usize)],
                );
            }

            create_grow!($body_name, $($grown_head_name)?, $($grown_body_name)?);
        }
    };
}

create_branch!(
    BranchHead4,
    BranchBody4,
    HeadTag::Branch4,
    ByteTable4,
    BranchHead8,
    BranchBody8
);
create_branch!(
    BranchHead8,
    BranchBody8,
    HeadTag::Branch8,
    ByteTable8,
    BranchHead16,
    BranchBody16
);
create_branch!(
    BranchHead16,
    BranchBody16,
    HeadTag::Branch16,
    ByteTable16,
    BranchHead32,
    BranchBody32
);
create_branch!(
    BranchHead32,
    BranchBody32,
    HeadTag::Branch32,
    ByteTable32,
    BranchHead64,
    BranchBody64
);
create_branch!(
    BranchHead64,
    BranchBody64,
    HeadTag::Branch64,
    ByteTable64,
    BranchHead128,
    BranchBody128
);
create_branch!(
    BranchHead128,
    BranchBody128,
    HeadTag::Branch128,
    ByteTable128,
    BranchHead256,
    BranchBody256
);
create_branch!(
    BranchHead256,
    BranchBody256,
    HeadTag::Branch256,
    ByteTable256
    ,,);

#[repr(C)]
struct PathBody<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    child: Head<KEY_LEN, Value>,
    rc: AtomicU16,
    fragment: [u8; BODY_FRAGMENT_LEN],
}

#[repr(C)]
struct PathHead<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; HEAD_FRAGMENT_LEN],
    end_depth: u8,
    body: NonNull<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>,
    phantom: PhantomData<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>,
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize>
    From<PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>> for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>) -> Self {
        sizeless_transmute::<PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>, Head<KEY_LEN, Value>>(
            head,
        )
    }
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize> From<Head<KEY_LEN, Value>>
    for PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: Head<KEY_LEN, Value>) -> Self {
        sizeless_transmute::<Head<KEY_LEN, Value>, PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>>(
            head,
        )
    }
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize> AsRef<Head<KEY_LEN, Value>>
    for PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn as_ref(&self) -> &Head<KEY_LEN, Value> {
        std::mem::transmute(self)
    }
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize>
    PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const TAG: HeadTag = match BODY_FRAGMENT_LEN {
        14 => HeadTag::Path14,
        30 => HeadTag::Path30,
        46 => HeadTag::Path46,
        62 => HeadTag::Path62,
        _ => panic!("invalid path length"),
    };
    const FRAGMENT_LEN: usize = BODY_FRAGMENT_LEN + HEAD_FRAGMENT_LEN;

    fn new(
        start_depth: usize,
        key: &[u8; KEY_LEN],
        child: Head<KEY_LEN, Value>,
    ) -> Head<KEY_LEN, Value> {
        unsafe {
            let end_depth = child.start_depth();
            let layout = Layout::new::<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>();
            let path_body = Global.allocate_zeroed(layout).unwrap().cast::<PathBody<
                KEY_LEN,
                Value,
                BODY_FRAGMENT_LEN,
            >>();
            *(path_body.as_mut()) = PathBody {
                child: child,
                rc: AtomicU16::new(1),
                fragment: mem::zeroed(),
            };

            copy_end(
                path_body.as_mut().fragment.as_mut_slice(),
                &key[..],
                end_depth as usize,
            );

            let actual_start_depth = max(
                start_depth as isize,
                end_depth as isize - Self::FRAGMENT_LEN as isize,
            ) as usize;

            let mut path_head = Self {
                tag: Self::TAG,
                start_depth: actual_start_depth as u8,
                fragment: mem::zeroed(),
                end_depth: end_depth,
                body: path_body,
                phantom: PhantomData,
            };

            copy_start(
                path_head.fragment.as_mut_slice(),
                &key[..],
                actual_start_depth,
            );

            path_head.into()
        }
    }

    fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        let actual_start_depth = max(
            start_depth as isize,
            self.end_depth as isize - Self::FRAGMENT_LEN as isize,
        ) as usize;
        self.start_depth = actual_start_depth as u8;
        copy_start(self.fragment.as_mut_slice(), &key[..], actual_start_depth);

        self.into()
    }

    fn peek(self, at_depth: usize) -> Option<u8> {
        if at_depth < self.start_depth as usize || self.end_depth as usize <= at_depth {
            return None;
        }
        if at_depth < self.start_depth as usize + HEAD_FRAGMENT_LEN {
            return Some(self.fragment[index_start(self.start_depth as usize, at_depth as usize)]);
        }
        return Some(
            self.body.as_ref().fragment[index_end(
                BODY_FRAGMENT_LEN,
                self.end_depth as usize,
                at_depth as usize,
            )],
        );
    }

    pub fn put(
        self,
        start_depth: usize,
        key: &[u8; KEY_LEN],
        value: Value,
        subtree_clone: bool,
    ) -> Head<KEY_LEN, Value> {
        let needs_clone = subtree_clone || self.body.as_ref().rc.load(Ordering::SeqCst) > 1;

        let mut branch_depth = start_depth;
        while branch_depth < self.end_depth as usize {
            if Some(key[branch_depth]) == self.peek(branch_depth) {
                branch_depth += 1
            } else {
                break;
            }
        }
        if branch_depth == self.end_depth as usize {
            // The entire infix matched with the key, i.e. branch_depth == self.branch_depth.
            let new_child =
                self.body
                    .as_ref()
                    .child
                    .put(self.end_depth as usize, key, value, needs_clone);
            if new_child.start_depth() != self.end_depth {
                return new_child.wrap_path(start_depth, key);
            }

            let mut cow = if needs_clone { self.clone() } else { self };
            cow.body.as_mut().child = new_child;

            return cow.into();
        }

        let sibling_leaf_node =
            LeafHead::new(branch_depth, key, value).wrap_path(branch_depth, key);

        let mut branch_head = BranchHead4::<KEY_LEN, Value>::new(start_depth, branch_depth, key);
        branch_head.insert(sibling_leaf_node);
        branch_head.insert(self.expand(branch_depth, key));

        return branch_head.wrap_path(start_depth, key);
    }
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize> Clone
    for PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn clone(&self) -> Self {
        Self {
            tag: Self::TAG,
            start_depth: self.start_depth,
            fragment: self.fragment,
            end_depth: self.end_depth,
            body: self.body,
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
enum HeadTag {
    Empty = 0,
    Branch4,
    Branch8,
    Branch16,
    Branch32,
    Branch64,
    Branch128,
    Branch256,
    Path14,
    Path30,
    Path46,
    Path62,
    Leaf,
}

macro_rules! dispatch {
    ($t:ty, $self:ident, $name:ident, $call:tt) => {
        {
            let mut $name = <Self as Into<$t>>::into($self);
            $call
        }
    };
}

macro_rules! dispatch_all {
        ($self:ident, $name:ident, $call:tt) => {
            match $self.tag {
                HeadTag::Empty => dispatch!(EmptyHead, $self, $name, $call),
                HeadTag::Leaf => dispatch!(LeafHead<KEY_LEN, Value>, $self, $name, $call),
                HeadTag::Path14 => dispatch!(PathHead<KEY_LEN, Value, 14>, $self, $name, $call),
                HeadTag::Path30 => dispatch!(PathHead<KEY_LEN, Value, 30>, $self, $name, $call),
                HeadTag::Path46 => dispatch!(PathHead<KEY_LEN, Value, 46>, $self, $name, $call),
                HeadTag::Path62 => dispatch!(PathHead<KEY_LEN, Value, 62>, $self, $name, $call),
                HeadTag::Branch4 => dispatch!(BranchHead4<KEY_LEN, Value>, $self, $name, $call),
                HeadTag::Branch8 => dispatch!(BranchHead8<KEY_LEN, Value>, $self, $name, $call),
                HeadTag::Branch16 => dispatch!(BranchHead16<KEY_LEN, Value>, $self, $name, $call),
                HeadTag::Branch32 => dispatch!(BranchHead32<KEY_LEN, Value>, $self, $name, $call),
                HeadTag::Branch64 => dispatch!(BranchHead64<KEY_LEN, Value>, $self, $name, $call),
                HeadTag::Branch128 => dispatch!(BranchHead128<KEY_LEN, Value>, $self, $name, $call),
                HeadTag::Branch256 => dispatch!(BranchHead256<KEY_LEN, Value>, $self, $name, $call),
            }
        };
    }

macro_rules! dispatch_ref {
    ($self:ident, $name:ident, $call:tt) => {
        match $self.tag {
            HeadTag::Empty => {
                let $name = AsRef::<&EmptyHead>::as_ref($self);
                $call
            }
            HeadTag::Leaf => {
                let $name = AsRef::<LeafHead<KEY_LEN, Value>>::as_ref($self);
                $call
            }
            HeadTag::Path14 => {
                let $name = AsRef::<PathHead<KEY_LEN, Value, 14>>::as_ref($self);
                $call
            }
            HeadTag::Path30 => {
                let $name = AsRef::<PathHead<KEY_LEN, Value, 30>>::as_ref($self);
                $call
            }
            HeadTag::Path46 => {
                let $name = AsRef::<PathHead<KEY_LEN, Value, 46>>::as_ref($self);
                $call
            }
            HeadTag::Path62 => {
                let $name = AsRef::<PathHead<KEY_LEN, Value, 62>>::as_ref($self);
                $call
            }
            HeadTag::Branch4 => {
                let $name = AsRef::<BranchHead4<KEY_LEN, Value>>::as_ref($self);
                $call
            }
            HeadTag::Branch8 => {
                let $name = AsRef::<BranchHead8<KEY_LEN, Value>>::as_ref($self);
                $call
            }
            HeadTag::Branch16 => {
                let $name = AsRef::<BranchHead16<KEY_LEN, Value>>::as_ref($self);
                $call
            }
            HeadTag::Branch32 => {
                let $name = AsRef::<BranchHead32<KEY_LEN, Value>>::as_ref($self);
                $call
            }
            HeadTag::Branch64 => {
                let $name = AsRef::<BranchHead64<KEY_LEN, Value>>::as_ref($self);
                $call
            }
            HeadTag::Branch128 => {
                let $name = AsRef::<BranchHead128<KEY_LEN, Value>>::as_ref($self);
                $call
            }
            HeadTag::Branch256 => {
                let $name = AsRef::<BranchHead256<KEY_LEN, Value>>::as_ref($self);
                $call
            }
        }
    };
}

#[repr(C)]
struct Head<const KEY_LEN: usize, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    key: u8,
    padding: [u8; 13],
    phantom: PhantomData<Value>,
}

impl<const KEY_LEN: usize, Value> Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn wrap_path(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let mut expanded = self.expand(start_depth, key);

        let actual_start_depth = expanded.start_depth() as usize;
        if start_depth == actual_start_depth {
            return expanded;
        }

        let path_length = actual_start_depth - start_depth;

        if path_length <= PathHead::<KEY_LEN, Value, 14>::FRAGMENT_LEN {
            return PathHead::<KEY_LEN, Value, 14>::new(start_depth, &key, expanded);
        }

        if path_length <= PathHead::<KEY_LEN, Value, 30>::FRAGMENT_LEN {
            return PathHead::<KEY_LEN, Value, 30>::new(start_depth, &key, expanded);
        }

        if path_length <= PathHead::<KEY_LEN, Value, 46>::FRAGMENT_LEN {
            return PathHead::<KEY_LEN, Value, 46>::new(start_depth, &key, expanded);
        }

        if path_length <= PathHead::<KEY_LEN, Value, 62>::FRAGMENT_LEN {
            return PathHead::<KEY_LEN, Value, 62>::new(start_depth, &key, expanded);
        }

        panic!("Fragment too long for path to hold.");
    }

    fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        match self.tag {
            HeadTag::Empty => panic!("Called `expand` on `Empty."),
            HeadTag::Leaf => dispatch!(LeafHead<KEY_LEN, Value>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Path14 => dispatch!(PathHead<KEY_LEN, Value, 14>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Path30 => dispatch!(PathHead<KEY_LEN, Value, 30>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Path46 => dispatch!(PathHead<KEY_LEN, Value, 46>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Path62 => dispatch!(PathHead<KEY_LEN, Value, 62>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Branch4 => dispatch!(BranchHead4<KEY_LEN, Value>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Branch8 => dispatch!(BranchHead8<KEY_LEN, Value>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Branch16 => dispatch!(BranchHead16<KEY_LEN, Value>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Branch32 => dispatch!(BranchHead32<KEY_LEN, Value>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Branch64 => dispatch!(BranchHead64<KEY_LEN, Value>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Branch128 => dispatch!(BranchHead128<KEY_LEN, Value>, self, head, { head.expand(start_depth, key) }),
            HeadTag::Branch256 => dispatch!(BranchHead256<KEY_LEN, Value>, self, head, { head.expand(start_depth, key) }),
        }
    }

    fn start_depth(&self) -> u8 {
        unsafe { self.start_depth }
    }

    fn insert(&mut self, child: Head<KEY_LEN, Value>) -> Head<KEY_LEN, Value> {
        unsafe {
            match self.tag {
                HeadTag::Branch4 => {
                    <Self as Into<BranchHead4<KEY_LEN, Value>>>::into(self)().insert(child)
                }
                HeadTag::Branch8 => {
                    <Self as Into<BranchHead8<KEY_LEN, Value>>>::into(self)().insert(child)
                }
                HeadTag::Branch16 => {
                    <Self as Into<BranchHead16<KEY_LEN, Value>>>::into(self)().insert(child)
                }
                HeadTag::Branch32 => {
                    <Self as Into<BranchHead32<KEY_LEN, Value>>>::into(self)().insert(child)
                }
                HeadTag::Branch64 => {
                    <Self as Into<BranchHead64<KEY_LEN, Value>>>::into(self)().insert(child)
                }
                HeadTag::Branch128 => {
                    <Self as Into<BranchHead128<KEY_LEN, Value>>>::into(self)().insert(child)
                }
                HeadTag::Branch256 => {
                    <Self as Into<BranchHead256<KEY_LEN, Value>>>::into(self)().insert(child)
                }
                _ => panic!("called insert on non-branch"),
            }
        }
    }

    pub fn put(self, start_depth: usize, key: &[u8; KEY_LEN], value: Value, cow: bool) -> Self {
        dispatch_all!(self, head, { head.put(start_depth, key, value) });
    }

    fn grow(self) -> Head<KEY_LEN, Value> {
        dispatch_all!(self, head, { head.grow() });
    }
}

unsafe impl<const KEY_LEN: usize, Value> ByteEntry for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn zeroed() -> Self {
        EmptyHead::new()
    }

    fn key(&self) -> Option<u8> {
        unsafe {
            if self.tag == EmptyHead::TAG {
                None
            } else {
                Some(self.key)
            }
        }
    }
}

impl<const KEY_LEN: usize, Value> Clone for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn clone(&self) -> Self {
        dispatch_ref!(self, head, { head.clone().into() });
    }
}

impl<const KEY_LEN: usize, Value> Drop for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn drop(&mut self) {
        dispatch_all!(self, head, { head });
    }
}

pub struct Tree<const KEY_LEN: usize, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    head: Head<KEY_LEN, Value>,
}

impl<const KEY_LEN: usize, Value> Tree<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const KEY_LEN_CHECK: usize = KEY_LEN - 64;

    pub fn new() -> Self {
        Tree {
            head: EmptyHead::new(),
        }
    }

    pub fn put(&mut self, key: [u8; KEY_LEN], value: Value) {
        self.head = self.head.put(0, &key, value, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head<64, ()>>(), 16);
        assert_eq!(mem::size_of::<Head<64, u64>>(), 16);
    }

    #[test]
    fn leaf_fragment_size() {
        let twig = Head::<64, ()> {
            leaf: ManuallyDrop::new(LeafHead::<64, ()> {
                tag: LeafHead::<64, ()>::TAG,
                fragment: unsafe { mem::zeroed() },
                start_depth: 0,
                value: (),
            }),
        };

        assert_eq!(unsafe { twig.leaf.fragment.len() }, 14);

        let leaf = Head::<64, u64> {
            leaf: ManuallyDrop::new(LeafHead::<64, u64> {
                tag: LeafHead::<64, u64>::TAG,
                fragment: unsafe { mem::zeroed() },
                start_depth: 0,
                value: 0,
            }),
        };
        assert_eq!(unsafe { leaf.leaf.fragment.len() }, 6);
    }

    #[test]
    fn empty_tree() {
        let tree = Tree::<64, ()>::new();
    }

    #[test]
    fn tree_insert_one() {
        const KEY_SIZE: usize = 64;
        let mut tree = Tree::<KEY_SIZE, ()>::new();
        let key = [0; KEY_SIZE];
        tree.put(key, ());
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable4<Head<64, ()>>>(), 64);
        assert_eq!(mem::size_of::<BranchBody4<64, ()>>(), 64 * 2);
        assert_eq!(mem::size_of::<BranchBody8<64, ()>>(), 64 * 3);
        assert_eq!(mem::size_of::<BranchBody16<64, ()>>(), 64 * 5);
        assert_eq!(mem::size_of::<BranchBody32<64, ()>>(), 64 * 9);
        assert_eq!(mem::size_of::<BranchBody64<64, ()>>(), 64 * 17);
        assert_eq!(mem::size_of::<BranchBody128<64, ()>>(), 64 * 33);
        assert_eq!(mem::size_of::<BranchBody256<64, ()>>(), 64 * 65);
    }

    #[test]
    fn fragment_size() {
        assert_eq!(mem::size_of::<PathBody<64, (), 14>>(), 16 * 2);
        assert_eq!(mem::size_of::<PathBody<64, (), 30>>(), 16 * 3);
        assert_eq!(mem::size_of::<PathBody<64, (), 46>>(), 16 * 4);
        assert_eq!(mem::size_of::<PathBody<64, (), 62>>(), 16 * 5);
    }
}
