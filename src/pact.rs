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

/*
impl<const KEY_LEN: usize, Value> EmptyHead<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    const TAG: HeadTag = HeadTag::Empty;
}
*/
/*
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

    fn peek(&self, at_depth: usize) -> Option<u8> {
        if KEY_LEN <= at_depth {
            return None; //TODO: do we need this vs. assert?
        }
        return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }
}
*/
/*
Path node:
fn peek(self, at_depth: usize) -> Option<u8> {
    if at_depth < self.start_depth as usize || self.end_depth as usize <= at_depth {
        return None;
    }
    if at_depth < self.start_depth as usize + HEAD_FRAGMENT_LEN {
        return Some(
            self.fragment[index_start(self.start_depth as usize, at_depth as usize)],
        );
    }
    return Some(
        self.body.as_ref().fragment[index_end(
            $body_fragment_len,
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
        let new_child = self.body.as_ref().child.put(
            self.end_depth as usize,
            key,
            value,
            needs_clone,
        );
        if new_child.start_depth() != self.end_depth {
            return new_child.wrap_path(start_depth, key);
        }

        let mut cow = if needs_clone { self.clone() } else { self };
        cow.body.as_mut().child = new_child;

        return cow.into();
    }

    let sibling_leaf_node =
        LeafHead::new(branch_depth, key, value).wrap_path(branch_depth, key);

    let mut branch_head =
        BranchHead4::<KEY_LEN, Value>::new(start_depth, branch_depth, key);
    branch_head.insert(sibling_leaf_node);
    branch_head.insert(self.expand(branch_depth, key));

    return branch_head.wrap_path(start_depth, key);
}
}

*/

/*
Branch:
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

*/

macro_rules! create_branchbody {
    ($body_name:ident, $table:tt) => {
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
    };
}

create_branchbody!(BranchBody4, ByteTable4);
create_branchbody!(BranchBody8, ByteTable8);
create_branchbody!(BranchBody16, ByteTable16);
create_branchbody!(BranchBody32, ByteTable32);
create_branchbody!(BranchBody64, ByteTable64);
create_branchbody!(BranchBody128, ByteTable128);
create_branchbody!(BranchBody256, ByteTable256);

macro_rules! create_pathbody {
    ($body_name:ident, $body_fragment_len:expr) => {
        #[repr(C)]
        struct $body_name<const KEY_LEN: usize, Value>
        where
            Value: SizeLimited<13> + Clone,
            [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            child: Head<KEY_LEN, Value>,
            rc: AtomicU16,
            fragment: [u8; $body_fragment_len],
        }

        impl<const KEY_LEN: usize, Value> $body_name<KEY_LEN, Value>
        where
            Value: SizeLimited<13> + Clone,
            [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
        {
            const FRAGMENT_LEN: usize = <Value as SizeLimited<13>>::UNUSED + 1;
        }
    };
}

create_pathbody!(PathBody14, 14);
create_pathbody!(PathBody30, 30);
create_pathbody!(PathBody46, 46);
create_pathbody!(PathBody62, 62);

#[repr(u8)]
enum Head<const KEY_LEN: usize, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    Empty {
        padding: [u8; 15],
    } = 0,
    Leaf {
        start_depth: u8,
        fragment: [u8; <Value as SizeLimited<13>>::UNUSED + 1],
        value: Value,
    },
    Path14 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<PathBody14<KEY_LEN, Value>>,
        phantom: PhantomData<PathBody14<KEY_LEN, Value>>,
    },
    Path30 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<PathBody30<KEY_LEN, Value>>,
        phantom: PhantomData<PathBody30<KEY_LEN, Value>>,
    },
    Path46 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<PathBody46<KEY_LEN, Value>>,
        phantom: PhantomData<PathBody46<KEY_LEN, Value>>,
    },
    Path62 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<PathBody62<KEY_LEN, Value>>,
        phantom: PhantomData<PathBody62<KEY_LEN, Value>>,
    },
    Branch4 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<BranchBody4<KEY_LEN, Value>>,
        phantom: PhantomData<BranchBody4<KEY_LEN, Value>>,
    },
    Branch8 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<BranchBody8<KEY_LEN, Value>>,
        phantom: PhantomData<BranchBody8<KEY_LEN, Value>>,
    },
    Branch16 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<BranchBody16<KEY_LEN, Value>>,
        phantom: PhantomData<BranchBody16<KEY_LEN, Value>>,
    },
    Branch32 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<BranchBody32<KEY_LEN, Value>>,
        phantom: PhantomData<BranchBody32<KEY_LEN, Value>>,
    },
    Branch64 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<BranchBody64<KEY_LEN, Value>>,
        phantom: PhantomData<BranchBody64<KEY_LEN, Value>>,
    },
    Branch128 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<BranchBody128<KEY_LEN, Value>>,
        phantom: PhantomData<BranchBody128<KEY_LEN, Value>>,
    },
    Branch256 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: NonNull<BranchBody256<KEY_LEN, Value>>,
        phantom: PhantomData<BranchBody256<KEY_LEN, Value>>,
    },
}

macro_rules! create_newpath {
    ($name:ident, $variant:ident, $body_name:ident) => {
        fn $name(start_depth: usize, key: &[u8; KEY_LEN], child: Self) -> Self {
            unsafe {
                let end_depth = child.start_depth();
                let layout = Layout::new::<$body_name<KEY_LEN, Value>>();
                let path_body = Global
                    .allocate_zeroed(layout)
                    .unwrap()
                    .cast::<$body_name<KEY_LEN, Value>>();
                *(path_body.as_mut()) = $body_name {
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
                    end_depth as isize
                        - ($body_name::FRAGMENT_LEN as isize + HEAD_FRAGMENT_LEN as isize),
                ) as usize;

                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

                Self::$variant {
                    start_depth: actual_start_depth as u8,
                    fragment: fragment,
                    end_depth: end_depth,
                    body: path_body,
                    phantom: PhantomData,
                }
            }
        }
    };
}

impl<const KEY_LEN: usize, Value> Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn newEmpty() -> Head<KEY_LEN, Value> {
        Self::Empty { padding: [0; 15] }
    }

    create_newpath!(newPath14, Path14, PathBody14);
    create_newpath!(newPath30, Path30, PathBody30);
    create_newpath!(newPath46, Path46, PathBody46);
    create_newpath!(newPath62, Path62, PathBody62);

    fn wrap_path(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let mut expanded = self.expand(start_depth, key);

        let actual_start_depth = expanded.start_depth() as usize;
        if start_depth == actual_start_depth {
            return expanded;
        }

        let path_length = actual_start_depth - start_depth;

        if path_length <= PathBody14::<KEY_LEN, Value>::FRAGMENT_LEN + HEAD_FRAGMENT_LEN {
            return Self::newPath14(start_depth, &key, expanded);
        }

        if path_length <= PathBody30::<KEY_LEN, Value>::FRAGMENT_LEN + HEAD_FRAGMENT_LEN {
            return Self::newPath30(start_depth, &key, expanded);
        }

        if path_length <= PathBody46::<KEY_LEN, Value>::FRAGMENT_LEN + HEAD_FRAGMENT_LEN {
            return Self::newPath46(start_depth, &key, expanded);
        }

        if path_length <= PathBody62::<KEY_LEN, Value>::FRAGMENT_LEN + HEAD_FRAGMENT_LEN {
            return Self::newPath62(start_depth, &key, expanded);
        }

        panic!("Fragment too long for path to hold.");
    }

    fn start_depth(&self) -> u8 {
        match self {
            Self::Empty { .. } => panic!("Called `start_depth` on `Empty`."),
            Self::Leaf { start_depth, .. } => start_depth,
            Self::Path14 { start_depth, .. } => start_depth,
            Self::Path30 { start_depth, .. } => start_depth,
            Self::Path46 { start_depth, .. } => start_depth,
            Self::Path62 { start_depth, .. } => start_depth,
            Self::Branch4 { start_depth, .. } => start_depth,
            Self::Branch8 { start_depth, .. } => start_depth,
            Self::Branch16 { start_depth, .. } => start_depth,
            Self::Branch32 { start_depth, .. } => start_depth,
            Self::Branch64 { start_depth, .. } => start_depth,
            Self::Branch128 { start_depth, .. } => start_depth,
            Self::Branch256 { start_depth, .. } => start_depth,
        }
    }

    fn expand(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN, Value> {
        macro_rules! pathexpand {
            ($end_depth:ident, $body:ident, $variant:ident, $fragment_len: expr) => {{
                let actual_start_depth = max(
                    start_depth as isize,
                    $end_depth as isize - $fragment_len as isize,
                ) as usize;

                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                //TODO Bugfix I think this might cause problems because we
                //need to copy from the body fragment.
                copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

                Self::$variant {
                    start_depth: actual_start_depth as u8,
                    fragment: fragment,
                    end_depth: $end_depth,
                    body: $body,
                    phantom: PhantomData,
                }
            }};
        }

        macro_rules! branchexpand {
            ($end_depth:ident, $body:ident, $variant:ident) => {{
                let actual_start_depth = max(
                    start_depth as isize,
                    $end_depth as isize - HEAD_FRAGMENT_LEN as isize,
                ) as usize;

                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

                Self::$variant {
                    start_depth: actual_start_depth as u8,
                    fragment: fragment,
                    end_depth: $end_depth,
                    body: $body,
                    phantom: PhantomData,
                }
            }};
        }

        match self {
            Self::Empty { .. } => panic!("Called `expand` on `Empty."),
            Self::Leaf { start_depth, value } => {
                let actual_start_depth = max(
                    start_depth as isize,
                    KEY_LEN as isize - { <Value as SizeLimited<13>>::UNUSED + 1 } as isize,
                ) as usize;

                //TODO Bugfix I think this might cause problems because we
                //need to copy from the old fragment.
                let mut fragment = [0; { <Value as SizeLimited<13>>::UNUSED + 1 }];
                copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

                Self::Leaf {
                    start_depth: actual_start_depth as u8,
                    fragment: fragment,
                    value: value,
                }
            }
            Self::Path14 {
                end_depth, body, ..
            } => pathexpand!(end_depth, body, Path14, {
                PathBody14::<KEY_LEN, Value>::FRAGMENT_LEN + HEAD_FRAGMENT_LEN
            }),
            Self::Path30 {
                end_depth, body, ..
            } => pathexpand!(end_depth, body, Path30, {
                PathBody30::<KEY_LEN, Value>::FRAGMENT_LEN + HEAD_FRAGMENT_LEN
            }),
            Self::Path46 {
                end_depth, body, ..
            } => pathexpand!(end_depth, body, Path46, {
                PathBody46::<KEY_LEN, Value>::FRAGMENT_LEN + HEAD_FRAGMENT_LEN
            }),
            Self::Path62 {
                end_depth, body, ..
            } => pathexpand!(end_depth, body, Path62, {
                PathBody62::<KEY_LEN, Value>::FRAGMENT_LEN + HEAD_FRAGMENT_LEN
            }),
            Self::Branch4 {
                end_depth, body, ..
            } => branchexpand!(end_depth, body, Path14),
            Self::Branch8 {
                end_depth, body, ..
            } => branchexpand!(end_depth, body, Path14),
            Self::Branch16 {
                end_depth, body, ..
            } => branchexpand!(end_depth, body, Path14),
            Self::Branch32 {
                end_depth, body, ..
            } => branchexpand!(end_depth, body, Path14),
            Self::Branch64 {
                end_depth, body, ..
            } => branchexpand!(end_depth, body, Path14),
            Self::Branch128 {
                end_depth, body, ..
            } => branchexpand!(end_depth, body, Path14),
            Self::Branch256 {
                end_depth, body, ..
            } => branchexpand!(end_depth, body, Path14),
        }
    }
}

/*
Head:
    fn insert(&mut self, child: Head<KEY_LEN, Value>) -> Head<KEY_LEN, Value> {
        unsafe {
            match self.tag {
                HeadTag::Branch4 => {
                    dispatch_mut!(BranchHead4<KEY_LEN, Value>, self, head, { head.insert(child) })
                }
                HeadTag::Branch8 => {
                    dispatch_mut!(BranchHead8<KEY_LEN, Value>, self, head, { head.insert(child) })
                }
                HeadTag::Branch16 => {
                    dispatch_mut!(BranchHead16<KEY_LEN, Value>, self, head, { head.insert(child) })
                }
                HeadTag::Branch32 => {
                    dispatch_mut!(BranchHead32<KEY_LEN, Value>, self, head, { head.insert(child) })
                }
                HeadTag::Branch64 => {
                    dispatch_mut!(BranchHead64<KEY_LEN, Value>, self, head, { head.insert(child) })
                }
                HeadTag::Branch128 => {
                    dispatch_mut!(BranchHead128<KEY_LEN, Value>, self, head, { head.insert(child) })
                }
                HeadTag::Branch256 => {
                    dispatch_mut!(BranchHead256<KEY_LEN, Value>, self, head, { head.insert(child) })
                }
                _ => panic!("Called insert on non-branch!"),
            }
        }
    }

    pub fn put(self, start_depth: usize, key: &[u8; KEY_LEN], value: Value, cow: bool) -> Self {
        match self {
            Empty {..} => Self::newLeaf(start_depth, key, value).wrap_path(start_depth, key),
            Leaf {..} => {
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

                let sibling_leaf_node = Self::newLeaf(branch_depth, key, value);

                let mut branch_head = Self::newBranch4(start_depth, branch_depth, key);
                branch_head.insert(sibling_leaf_node);
                branch_head.insert(self.expand(branch_depth, key));

                return branch_head.wrap_path(start_depth, key);
            },
            HeadTag::Path14 => dispatch!(PathHead14<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Path30 => dispatch!(PathHead30<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Path46 => dispatch!(PathHead46<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Path62 => dispatch!(PathHead62<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Branch4 => dispatch!(BranchHead4<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Branch8 => dispatch!(BranchHead8<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Branch16 => dispatch!(BranchHead16<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Branch32 => dispatch!(BranchHead32<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Branch64 => dispatch!(BranchHead64<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Branch128 => dispatch!(BranchHead128<KEY_LEN, Value>, $self, $name, $call),
            HeadTag::Branch256 => dispatch!(BranchHead256<KEY_LEN, Value>, $self, $name, $call),
        }
        dispatch_all!(self, head, { head.put(start_depth, key, value) });
    }

    fn grow(self) -> Head<KEY_LEN, Value> {
        dispatch_all!(self, head, { head.grow() });
    }
*/

impl<const KEY_LEN: usize, Value> Clone for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn clone(&self) -> Self {
        match self {
            Self::Empty { .. } => Self::newEmpty(),
            Self::Leaf {
                start_depth,
                fragment,
                value,
            } => Self::Leaf {
                start_depth,
                fragment,
                value,
            },
            Self::Path14 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Path14 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Path30 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Path30 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Path46 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Path46 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Path62 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Path62 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Branch4 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Branch4 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Branch8 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Branch8 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Branch16 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Branch16 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Branch32 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Branch32 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Branch64 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Branch64 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Branch128 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Branch128 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
            Self::Branch256 {
                start_depth,
                fragment,
                end_depth,
                body,
            } => Self::Branch256 {
                start_depth,
                fragment,
                end_depth,
                body,
                phantom: PhantomData,
            },
        }
    }
}

impl<const KEY_LEN: usize, Value> Drop for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn drop(&mut self) {}
}

unsafe impl<const KEY_LEN: usize, Value> ByteEntry for Head<KEY_LEN, Value>
where
    Value: SizeLimited<13> + Clone,
    [u8; <Value as SizeLimited<13>>::UNUSED + 1]: Sized,
{
    fn zeroed() -> Self {
        Self::newEmpty()
    }

    fn key(&self) -> Option<u8> {
        match self {
            Self::Empty { .. } => None,
            Self::Leaf { fragment, .. } => fragment[0],
            Self::Path14 { fragment, .. } => fragment[0],
            Self::Path30 { fragment, .. } => fragment[0],
            Self::Path46 { fragment, .. } => fragment[0],
            Self::Path62 { fragment, .. } => fragment[0],
            Self::Branch4 { fragment, .. } => fragment[0],
            Self::Branch8 { fragment, .. } => fragment[0],
            Self::Branch16 { fragment, .. } => fragment[0],
            Self::Branch32 { fragment, .. } => fragment[0],
            Self::Branch64 { fragment, .. } => fragment[0],
            Self::Branch128 { fragment, .. } => fragment[0],
            Self::Branch256 { fragment, .. } => fragment[0],
        }
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
            head: Head::<KEY_LEN, Value>::newEmpty(),
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
        assert_eq!(mem::size_of::<PathBody14<64, ()>>(), 16 * 2);
        assert_eq!(mem::size_of::<PathBody30<64, ()>>(), 16 * 3);
        assert_eq!(mem::size_of::<PathBody46<64, ()>>(), 16 * 4);
        assert_eq!(mem::size_of::<PathBody62<64, ()>>(), 16 * 5);
    }
}
