use crate::bitset::ByteBitset;
use crate::bytetable::{ByteEntry, ByteTable};
//use siphasher::sip128::{Hasher128, SipHasher24};
use std::alloc::{alloc, /* dealloc, realloc, */Layout};
use std::cmp::{max, min};
use std::marker::PhantomData;
use std::mem;
use std::mem::{ MaybeUninit };
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::{ AtomicU16, Ordering};
use std::mem::ManuallyDrop;
use std::fmt::Debug;
use std::fmt;

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
    let source_range = & source[end_index - used_len..end_index];
    target_range.copy_from_slice(source_range);
}

fn copy_start(target: &mut [u8], source: &[u8], start_index: usize) {
    let target_len = target.len();
    let source_len = source.len();
    let used_len = min(source_len - start_index as usize, target_len);
    let target_range = &mut target[0..used_len];
    let source_range = & source[start_index..start_index as usize + used_len];
    target_range.copy_from_slice(source_range);
}

#[derive(Debug)]
#[repr(C)]
struct UnknownHead {
    tag: HeadTag,
    start_depth: u8,
    key: u8,
    padding: [u8; 13],
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct EmptyHead {
    tag: HeadTag,
    padding: [u8; 15],
}

impl<const KEY_LEN: usize, Value> From<EmptyHead> for Head<KEY_LEN, Value> 
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: EmptyHead) -> Self {
        sizeless_transmute::<EmptyHead, Head<KEY_LEN, Value>>(head)
    }
}

impl EmptyHead {
    const TAG: HeadTag = HeadTag::Empty;

    fn new<const KEY_LEN: usize, Value>() -> Head<KEY_LEN, Value>
    where
        Value: SizeLimited<13> + Clone + Debug,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized
    {
        (Self {
            tag: Self::TAG,
            padding: mem::zeroed()
        }).into()
    }

    fn put<const KEY_LEN: usize, Value>(self, start_depth: usize, key: &[u8; KEY_LEN], value: Value) -> Head<KEY_LEN, Value>
    where
        Value: SizeLimited<13> + Clone + Debug,
        [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized
    {
        LeafHead::<KEY_LEN, Value>::new(start_depth, key, value)
        .wrap_path(start_depth, key)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct LeafHead<const KEY_LEN: usize, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; <Value as SizeLimited<13>>::UNUSED + 1],
    value: Value,
}

impl<const KEY_LEN: usize, Value> From<LeafHead<KEY_LEN, Value>> for Head<KEY_LEN, Value> 
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: LeafHead<KEY_LEN, Value>) -> Self {
        sizeless_transmute::<LeafHead<KEY_LEN, Value>, Head<KEY_LEN, Value>>(head)
    }
}

impl<const KEY_LEN: usize, Value> LeafHead<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
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

        copy_start(leaf_head.fragment.as_mut_slice(), &key[..], actual_start_depth);

        leaf_head.into()
    }

    fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        let actual_start_depth = max(start_depth as isize, KEY_LEN as isize - Self::FRAGMENT_LEN as isize) as usize;
        self.start_depth = actual_start_depth as u8;
        copy_start(self.fragment.as_mut_slice(), &key[..], actual_start_depth);

        self.into()
    }

    fn peek(& self, at_depth: usize) -> Option<u8> {
        if KEY_LEN <= at_depth {
            return None; //TODO: do we need this vs. assert?
        }
        return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }

    pub fn put(self, start_depth: usize, key: &[u8; KEY_LEN], value: Value) -> Head<KEY_LEN, Value> {
        let mut branch_depth = start_depth;
        while branch_depth < KEY_LEN {
            if Some(key[branch_depth]) == self.peek(branch_depth) {
                branch_depth += 1
            } else {
                break;
            }
        }
        if branch_depth == KEY_LEN {
            return Head::<KEY_LEN, Value> {
                leaf: ManuallyDrop::new(self)
            };
        }

        let sibling_leaf_node = LeafHead::new(branch_depth, key, value);

        let mut branch_head = BranchHead::<KEY_LEN, Value, 1>::new(start_depth, branch_depth, key);
        (&mut branch_head.branch1).insert(sibling_leaf_node);
        (&mut branch_head.branch1).insert(self.expand(branch_depth, key));

        return branch_head.wrap_path(start_depth, key);
    }
}

#[derive(Debug)]
#[repr(C)]
struct BranchBody<const KEY_LEN: usize, Value, const TABLE_SIZE: usize>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    leaf_count: u64,
    rc: AtomicU16,
    segment_count: u32, //TODO: increase this to a u48
    node_hash: u128,
    child_set: ByteBitset,
    children: ByteTable<TABLE_SIZE, Head<KEY_LEN, Value>>,
}

#[derive(Debug)]
#[repr(C)]
struct BranchHead<const KEY_LEN: usize, Value, const TABLE_SIZE: usize>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; HEAD_FRAGMENT_LEN],
    end_depth: u8,
    body: NonNull<BranchBody<KEY_LEN, Value, TABLE_SIZE>>,
    phantom: PhantomData<BranchBody<KEY_LEN, Value, TABLE_SIZE>>,
}

impl<const KEY_LEN: usize, Value, const TABLE_SIZE: usize> From<BranchHead<KEY_LEN, Value, TABLE_SIZE>> for Head<KEY_LEN, Value> 
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: BranchHead<KEY_LEN, Value, TABLE_SIZE>) -> Self {
        sizeless_transmute::<BranchHead<KEY_LEN, Value, TABLE_SIZE>, Head<KEY_LEN, Value>>(head)
    }
}

impl<const KEY_LEN: usize, Value, const TABLE_SIZE: usize> Clone
    for BranchHead<KEY_LEN, Value, TABLE_SIZE>
where
    Value: SizeLimited<13> + Clone + Debug,
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

impl<const KEY_LEN: usize, Value, const TABLE_SIZE: usize> BranchHead<KEY_LEN, Value, TABLE_SIZE>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const TAG: HeadTag = match TABLE_SIZE {
        1 => HeadTag::Branch1,
        2 => HeadTag::Branch2,
        4 => HeadTag::Branch4,
        8 => HeadTag::Branch8,
        16 => HeadTag::Branch16,
        32 => HeadTag::Branch32,
        64 => HeadTag::Branch64,
        _ => panic!("invalid path length"),
    };
    const FRAGMENT_LEN: usize = HEAD_FRAGMENT_LEN;

    fn new(start_depth: usize, branch_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        unsafe {
            let layout = Layout::new::<BranchBody<KEY_LEN, Value, TABLE_SIZE>>();
            let branch_body = alloc(layout) as *mut BranchBody<KEY_LEN, Value, TABLE_SIZE>;
            if branch_body.is_null() {
                panic!("Alloc error!");
            }
            branch_body.write(BranchBody {
                leaf_count: 0,
                rc: AtomicU16::new(1),
                segment_count: 0,
                node_hash: 0,
                child_set: ByteBitset::new_empty(),
                children: ByteTable::new(),
            });

            let actual_start_depth = max(start_depth as isize, branch_depth as isize - Self::FRAGMENT_LEN as isize) as usize;

            let mut branch_head = Self {
                tag: Self::TAG,
                start_depth: actual_start_depth as u8,
                fragment: mem::zeroed(),
                end_depth: branch_depth as u8,
                body: NonNull::new_unchecked(branch_body),
                phantom: PhantomData
            };
            
            copy_start(branch_head.fragment.as_mut_slice(), &key[..], actual_start_depth);

            branch_head.into()
        }
    }

    fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        let actual_start_depth = max(start_depth as isize, self.end_depth as isize - Self::FRAGMENT_LEN as isize) as usize;
        self.start_depth = actual_start_depth as u8;
        copy_start(self.fragment.as_mut_slice(), &key[..], actual_start_depth);

        self.into()
    }

    fn insert(&mut self, child: Head<KEY_LEN, Value>) {
        self.body.as_mut().children.put(child);
    }

    fn put(self, start_depth: usize, key: &[u8; KEY_LEN], value: Value, subtree_clone: bool) -> Head<KEY_LEN, Value> {
//>
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
            let new_child = self.body.as_ref().child
                                .put(self.end_depth as usize, key, value, needs_clone);
            if new_child.start_depth() != self.end_depth {
                return new_child.wrap_path(start_depth, key);
            }

            let mut cow = if needs_clone { self.clone() } else { self };
            cow.body.as_mut().child = new_child;
            
            return cow.into()
        }

        let sibling_leaf_node = LeafHead::new(branch_depth, key, value).wrap_path(branch_depth, key);

        let mut branch_head = BranchHead::<KEY_LEN, Value, 1>::new(start_depth, branch_depth, key);
        (&mut branch_head.branch1).insert(sibling_leaf_node);
        (&mut branch_head.branch1).insert(self.expand(branch_depth, key));

        return branch_head.wrap_path(start_depth, key);


        //////

        let needs_clone = subtree_clone || self.body.as_ref().rc.load(Ordering::SeqCst) > 1;
//>
        var branch_depth = start_depth;
        while (branch_depth < self.branch_depth) : (branch_depth += 1) {
            if (key[branch_depth] != self.peek(branch_depth).?) break;
        } else {
            // The entire compressed infix above this node matched with the key.
            const byte_key = key[branch_depth];
            if (self.hasBranch(byte_key)) {
                // The node already has a child branch with the same byte byte_key as the one in the key.
                const old_child = self.getBranch(byte_key);
                const old_child_hash = old_child.hash(key);
                const old_child_leaf_count = old_child.count();
                const old_child_segment_count = old_child.segmentCount(branch_depth);
                const new_child = try old_child.put(branch_depth, key, value, single_owner, allocator);
                const new_child_hash = new_child.hash(key);

                const new_hash = self.body.node_hash.update(old_child_hash, new_child_hash);
                const new_leaf_count = self.body.leaf_count - old_child_leaf_count + new_child.count();
                const new_segment_count = self.body.segment_count - old_child_segment_count + new_child.segmentCount(branch_depth);

                var self_or_copy = self;
                if (!single_owner) {
                    self_or_copy = try self.copy(allocator);
                    old_child.rel(allocator);
                }
                self_or_copy.body.node_hash = new_hash;
                self_or_copy.body.leaf_count = new_leaf_count;
                self_or_copy.body.segment_count = new_segment_count;

                self_or_copy.updateBranch(new_child);
                return @bitCast(Node, self_or_copy);
            } else {
                const new_child_node = try WrapInfixNode(branch_depth, key, InitLeaf(branch_depth, key, value), allocator);

                var self_or_copy = if (single_owner) self else try self.copy(allocator);

                var displaced = self_or_copy.createBranch(new_child_node, branch_depth, key);
                var grown = @bitCast(Node, self_or_copy);
                while (displaced) |entry| {
                    grown = try grown.grow(allocator);
                    displaced = grown.reinsertBranch(entry);
                }
                return grown;
            }
        }

        const sibling_leaf_node = try WrapInfixNode(branch_depth, key, InitLeaf(branch_depth, key, value), allocator);

        return try BranchNodeBase.initBranch(start_depth, branch_depth, key, sibling_leaf_node, self.initAt(branch_depth, key), allocator);
    }
}

#[derive(Debug)]
#[repr(C)]
struct PathBody<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    child: Head<KEY_LEN, Value>,
    rc: AtomicU16,
    fragment: [u8; BODY_FRAGMENT_LEN],
}

#[derive(Debug)]
#[repr(C)]
struct PathHead<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; HEAD_FRAGMENT_LEN],
    end_depth: u8,
    body: NonNull<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>,
    phantom: PhantomData<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>,
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize> From<PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>> for Head<KEY_LEN, Value> 
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn from(head: PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>) -> Self {
        sizeless_transmute::<PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>, Head<KEY_LEN, Value>>(head)
    }
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize> PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone + Debug,
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

    fn new(start_depth: usize, key: &[u8; KEY_LEN], child: Head<KEY_LEN, Value>) -> Head<KEY_LEN, Value> {
        unsafe {
            let end_depth = child.start_depth();
            let layout = Layout::new::<PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>>();
            let path_body = alloc(layout) as *mut PathBody<KEY_LEN, Value, BODY_FRAGMENT_LEN>;
            if path_body.is_null() {
                panic!("Alloc error!");
            }
            path_body.write(PathBody {
                child: child,
                rc: AtomicU16::new(1),
                fragment: mem::zeroed(),
            });

            copy_end((*path_body).fragment.as_mut_slice(), &key[..], end_depth as usize);

            let actual_start_depth = max(start_depth as isize, end_depth as isize - Self::FRAGMENT_LEN as isize) as usize;

            let mut path_head = Self {
                tag: Self::TAG,
                start_depth: actual_start_depth as u8,
                fragment: mem::zeroed(),
                end_depth: end_depth,
                body: NonNull::new_unchecked(path_body),
                phantom: PhantomData
            };
            
            copy_start(path_head.fragment.as_mut_slice(), &key[..], actual_start_depth);

            path_head.into()
        }
    }

    fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        let actual_start_depth = max(start_depth as isize, self.end_depth as isize - Self::FRAGMENT_LEN as isize) as usize;
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
        return Some(self.body.as_ref().fragment[index_end(BODY_FRAGMENT_LEN, self.end_depth as usize, at_depth as usize)]);
    }

    pub fn put(self, start_depth: usize, key: &[u8; KEY_LEN], value: Value, subtree_clone: bool) -> Head<KEY_LEN, Value> {
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
            let new_child = self.body.as_ref().child
                                .put(self.end_depth as usize, key, value, needs_clone);
            if new_child.start_depth() != self.end_depth {
                return new_child.wrap_path(start_depth, key);
            }

            let mut cow = if needs_clone { self.clone() } else { self };
            cow.body.as_mut().child = new_child;
            
            return cow.into()
        }

        let sibling_leaf_node = LeafHead::new(branch_depth, key, value).wrap_path(branch_depth, key);

        let mut branch_head = BranchHead::<KEY_LEN, Value, 1>::new(start_depth, branch_depth, key);
        (&mut branch_head.branch1).insert(sibling_leaf_node);
        (&mut branch_head.branch1).insert(self.expand(branch_depth, key));

        return branch_head.wrap_path(start_depth, key);
    }
}

impl<const KEY_LEN: usize, Value, const BODY_FRAGMENT_LEN: usize> Clone
    for PathHead<KEY_LEN, Value, BODY_FRAGMENT_LEN>
where
    Value: SizeLimited<13> + Clone + Debug,
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
    Branch1,
    Branch2,
    Branch4,
    Branch8,
    Branch16,
    Branch32,
    Branch64,
    Path14,
    Path30,
    Path46,
    Path62,
    Leaf,
}

#[repr(C)]
union Head<const KEY_LEN: usize, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    any: [MaybeUninit<u8>; 16],
    unknown: ManuallyDrop<UnknownHead>,
    empty: ManuallyDrop<EmptyHead>,
    branch1: ManuallyDrop<BranchHead<KEY_LEN, Value, 1>>,
    branch2: ManuallyDrop<BranchHead<KEY_LEN, Value, 2>>,
    branch4: ManuallyDrop<BranchHead<KEY_LEN, Value, 4>>,
    branch8: ManuallyDrop<BranchHead<KEY_LEN, Value, 8>>,
    branch16: ManuallyDrop<BranchHead<KEY_LEN, Value, 16>>,
    branch32: ManuallyDrop<BranchHead<KEY_LEN, Value, 32>>,
    branch64: ManuallyDrop<BranchHead<KEY_LEN, Value, 64>>,
    path14: ManuallyDrop<PathHead<KEY_LEN, Value, 14>>,
    path30: ManuallyDrop<PathHead<KEY_LEN, Value, 30>>,
    path46: ManuallyDrop<PathHead<KEY_LEN, Value, 46>>,
    path62: ManuallyDrop<PathHead<KEY_LEN, Value, 62>>,
    leaf: ManuallyDrop<LeafHead<KEY_LEN, Value>>,
}

impl<const KEY_LEN: usize, Value> Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn wrap_path(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        dbg!(&self);
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

    fn expand(&mut self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => panic!("Called `expand` on `Empty."),
                HeadTag::Leaf => self.leaf.expand(start_depth, key),
                HeadTag::Path14 => self.path14.expand(start_depth, key),
                HeadTag::Path30 => self.path30.expand(start_depth, key),
                HeadTag::Path46 => self.path46.expand(start_depth, key),
                HeadTag::Path62 => self.path62.expand(start_depth, key),
                HeadTag::Branch1 => self.branch1.expand(start_depth, key),
                HeadTag::Branch2 => self.branch2.expand(start_depth, key),
                HeadTag::Branch4 => self.branch4.expand(start_depth, key),
                HeadTag::Branch8 => self.branch8.expand(start_depth, key),
                HeadTag::Branch16 => self.branch16.expand(start_depth, key),
                HeadTag::Branch32 => self.branch32.expand(start_depth, key),
                HeadTag::Branch64 => self.branch64.expand(start_depth, key),
            }
        }
    }

    fn start_depth(&self) -> u8 {
        unsafe {self.unknown.start_depth}
    }

    pub fn put(self, start_depth: usize, key: &[u8; KEY_LEN], value: Value, cow: bool) -> Self {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.put(start_depth, key, value),
                HeadTag::Leaf => self.leaf.put(start_depth, key, value),
                HeadTag::Path14 => self.path14.put(start_depth, key, value, cow),
                HeadTag::Path30 => self.path30.put(start_depth, key, value, cow),
                HeadTag::Path46 => self.path46.put(start_depth, key, value, cow),
                HeadTag::Path62 => self.path62.put(start_depth, key, value, cow),
                HeadTag::Branch1 => self.branch1.put(start_depth, key, value, cow),
                HeadTag::Branch2 => self.branch2.put(start_depth, key, value, cow),
                HeadTag::Branch4 => self.branch4.put(start_depth, key, value, cow),
                HeadTag::Branch8 => self.branch8.put(start_depth, key, value, cow),
                HeadTag::Branch16 => self.branch16.put(start_depth, key, value, cow),
                HeadTag::Branch32 => self.branch32.put(start_depth, key, value, cow),
                HeadTag::Branch64 => self.branch64.put(start_depth, key, value, cow),
            }
        }
    }
}

impl<const KEY_LEN: usize, Value> fmt::Debug for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.fmt(f),
                HeadTag::Leaf => self.leaf.fmt(f),
                HeadTag::Path14 => self.path14.fmt(f),
                HeadTag::Path30 => self.path30.fmt(f),
                HeadTag::Path46 => self.path46.fmt(f),
                HeadTag::Path62 => self.path62.fmt(f),
                HeadTag::Branch1 => self.branch1.fmt(f),
                HeadTag::Branch2 => self.branch2.fmt(f),
                HeadTag::Branch4 => self.branch4.fmt(f),
                HeadTag::Branch8 => self.branch8.fmt(f),
                HeadTag::Branch16 => self.branch16.fmt(f),
                HeadTag::Branch32 => self.branch32.fmt(f),
                HeadTag::Branch64 => self.branch64.fmt(f),
            }
        }
    }
}

unsafe impl<const KEY_LEN: usize, Value> ByteEntry for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn zeroed() -> Self {
        EmptyHead::new()
    }

    fn key(&self) -> Option<u8> {
        unsafe {
            if self.unknown.tag == EmptyHead::TAG {
                None
            } else {
                Some(self.unknown.key)
            }
        }
    }
}

impl<const KEY_LEN: usize, Value> Clone for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn clone(&self) -> Self {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => Self { empty: self.empty.clone() },
                HeadTag::Leaf => Self { leaf: self.leaf.clone() },
                HeadTag::Path14 => Self { path14: self.path14.clone() },
                HeadTag::Path30 => Self { path30: self.path30.clone() },
                HeadTag::Path46 => Self { path46: self.path46.clone() },
                HeadTag::Path62 => Self { path62: self.path62.clone() },
                HeadTag::Branch1 => Self { branch1: self.branch1.clone() },
                HeadTag::Branch2 => Self { branch2: self.branch2.clone() },
                HeadTag::Branch4 => Self { branch4: self.branch4.clone() },
                HeadTag::Branch8 => Self { branch8: self.branch8.clone() },
                HeadTag::Branch16 => Self { branch16: self.branch16.clone() },
                HeadTag::Branch32 => Self { branch32: self.branch32.clone() },
                HeadTag::Branch64 => Self { branch64: self.branch64.clone() },
            }
        }
    }
}

impl<const KEY_LEN: usize, Value> Drop for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn drop(&mut self) {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => ManuallyDrop::drop(&mut self.empty),
                HeadTag::Leaf => ManuallyDrop::drop(&mut self.leaf),
                HeadTag::Path14 => ManuallyDrop::drop(&mut self.path14),
                HeadTag::Path30 => ManuallyDrop::drop(&mut self.path30),
                HeadTag::Path46 => ManuallyDrop::drop(&mut self.path46),
                HeadTag::Path62 => ManuallyDrop::drop(&mut self.path62),
                HeadTag::Branch1 => ManuallyDrop::drop(&mut self.branch1),
                HeadTag::Branch2 => ManuallyDrop::drop(&mut self.branch2),
                HeadTag::Branch4 => ManuallyDrop::drop(&mut self.branch4),
                HeadTag::Branch8 => ManuallyDrop::drop(&mut self.branch8),
                HeadTag::Branch16 => ManuallyDrop::drop(&mut self.branch16),
                HeadTag::Branch32 => ManuallyDrop::drop(&mut self.branch32),
                HeadTag::Branch64 => ManuallyDrop::drop(&mut self.branch64),
            }
        }
    }
}

pub struct Tree<const KEY_LEN: usize, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    head: Head<KEY_LEN, Value>,
}

impl<const KEY_LEN: usize, Value> Tree<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone + Debug,
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
            })
        };

        assert_eq!(unsafe { twig.leaf.fragment.len() }, 14);

        let leaf = Head::<64, u64> {
            leaf: ManuallyDrop::new(LeafHead::<64, u64> {
                tag: LeafHead::<64, u64>::TAG,
                fragment: unsafe { mem::zeroed() },
                start_depth: 0,
                value: 0,
            })
        };
        assert_eq!(unsafe {leaf.leaf.fragment.len()}, 6);
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
        assert_eq!(mem::size_of::<ByteTable<1, Head<64, ()>>>(), 64);
        assert_eq!(mem::size_of::<BranchBody<64, (), 1>>(), 64 * 2);
        assert_eq!(mem::size_of::<BranchBody<64, (), 2>>(), 64 * 3);
        assert_eq!(mem::size_of::<BranchBody<64, (), 4>>(), 64 * 5);
        assert_eq!(mem::size_of::<BranchBody<64, (), 8>>(), 64 * 9);
        assert_eq!(mem::size_of::<BranchBody<64, (), 16>>(), 64 * 17);
        assert_eq!(mem::size_of::<BranchBody<64, (), 32>>(), 64 * 33);
        assert_eq!(mem::size_of::<BranchBody<64, (), 64>>(), 64 * 65);
    }

    #[test]
    fn fragment_size() {
        assert_eq!(mem::size_of::<PathBody<64, (), 14>>(), 16 * 2);
        assert_eq!(mem::size_of::<PathBody<64, (), 30>>(), 16 * 3);
        assert_eq!(mem::size_of::<PathBody<64, (), 46>>(), 16 * 4);
        assert_eq!(mem::size_of::<PathBody<64, (), 62>>(), 16 * 5);
    }
}
