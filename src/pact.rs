use crate::bitset::ByteBitset;
use crate::bytetable;
use crate::bytetable::*;
use crate::query::{ ByteCursor }; //CursorIterator
use std::sync::Once;
use std::cmp::{max, min};
use std::mem;
use std::sync::Arc;
use rand::thread_rng;
use rand::RngCore;
//use core::hash::Hasher;
//use siphasher::sip128::{Hasher128, SipHasher24};
use std::mem::ManuallyDrop;
use std::fmt::Debug;
use std::fmt;
use std::mem::{ MaybeUninit, transmute };

static mut SIP_KEY: [u8; 16] = [0; 16];
static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        bytetable::init();
        
        let mut rng = thread_rng();
        unsafe {
            rng.fill_bytes(&mut SIP_KEY[..]);
        }
    });
}

const HEAD_SIZE: usize = 16;
const HEAD_FRAGMENT_LEN: usize = 5;
const LEAF_FRAGMENT_LEN: usize = 14;

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
*/

macro_rules! create_branch {
    ($name:ident, $body_name:ident, $table:tt) => {
        #[derive(Clone, Debug)]
        #[repr(C)]
        pub struct $body_name<const KEY_LEN: usize> {
            leaf_count: u64,
            //rc: AtomicU16,
            //segment_count: u32, //TODO: increase this to a u48
            //hash: u128,
            child_set: ByteBitset,
            child_table: $table<Head<KEY_LEN>>,
        }

        #[derive(Clone, Debug)]
        #[repr(C)]
        pub struct $name<const KEY_LEN: usize> {
            tag: HeadTag,
            start_depth: u8,
            fragment: [u8; HEAD_FRAGMENT_LEN],
            end_depth: u8,
            body: Arc<$body_name<KEY_LEN>>,
        }

        impl<const KEY_LEN: usize> From<$name<KEY_LEN>> for Head<KEY_LEN> {
            fn from(head: $name<KEY_LEN>) -> Self {
                unsafe {
                    transmute(head)
                }
            }
        }

        impl<const KEY_LEN: usize> $name<KEY_LEN> {
            fn new(start_depth: usize, end_depth: usize, key: &[u8; KEY_LEN]) -> Self {            
                    let actual_start_depth = max(
                        start_depth as isize,
                        end_depth as isize - HEAD_FRAGMENT_LEN as isize,
                    ) as usize;
            
                    let mut fragment = [0; HEAD_FRAGMENT_LEN];
                    copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);
            
                    Self {
                            tag: HeadTag::$name,
                            start_depth: actual_start_depth as u8,
                            fragment: fragment,
                            end_depth: end_depth as u8,
                            body: Arc::new($body_name {
                                leaf_count: 0,
                                //rc: AtomicU16::new(1),
                                //segment_count: 0,
                                //node_hash: 0,
                                child_set: ByteBitset::new_empty(),
                                child_table: $table::new(),
                            }),
                    }
            }

            pub fn count(&self) -> u64 {
                self.body.leaf_count
            }

            fn insert(&mut self, child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                let inner = Arc::make_mut(&mut self.body);
                inner.child_set.set(child.key().expect("leaf should have a byte key"));
                inner.leaf_count += child.count();
                inner.child_table.put(child)
            }

            fn reinsert(&mut self, child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                let inner = Arc::make_mut(&mut self.body);
                inner.child_table.put(child)
            }

            fn peek(&self, at_depth: usize) -> Option<u8> {
                if at_depth < self.start_depth as usize || self.end_depth as usize <= at_depth {
                    return None;
                }
                return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
            }
        
            fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
                if at_depth == self.end_depth as usize {
                    *result_set = self.body.child_set;
                    return;
                }
        
                result_set.unset_all();
                if let Some(byte_key) = self.peek(at_depth) {
                    result_set.set(byte_key);
                    return;
                }
            }
        
            fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let mut branch_depth = self.start_depth as usize;
                while Some(key[branch_depth]) == self.peek(branch_depth) {
                    branch_depth += 1;
                }

                if branch_depth == self.end_depth as usize {
                // The entire fragment matched with the key.
                
                    let byte_key = key[branch_depth];
                    if self.body.child_set.is_set(byte_key) {
                    // We already have a child with the same byte as the key.

                        let new_body = Arc::make_mut(&mut self.body);
                        let old_child = new_body.child_table.get_mut(byte_key).expect("table content should match child set content");
                            //let old_child_hash = old_child.hash(key);
                            //let old_child_segment_count = old_child.segmentCount(branch_depth);
                            //let new_child_hash = new_child.hash(key);
                        let old_child_leaf_count = old_child.count();
                        let new_child = old_child.put(key);

                            //let new_hash = self.body.node_hash.update(old_child_hash, new_child_hash);
                            //let new_segment_count = self.body.segment_count - old_child_segment_count + new_child.segmentCount(branch_depth);

                            //new_body.node_hash = new_hash;
                            //new_body.segment_count = new_segment_count;
                        new_body.leaf_count = (new_body.leaf_count - old_child_leaf_count as u64) + new_child.count() as u64;
                        new_body.child_table.put(new_child);

                        return self.clone().into();
                    } else {
                    // We don't have a child with the byte of the key.

                        let mut displaced = self.insert(Head::from(Leaf::new(branch_depth, key)).wrap_path(branch_depth, key));
                        if None == displaced.key() {
                            Head::from(self.clone());
                        }

                        let mut new_self = Head::from(self.clone());
                        while None != displaced.key() {
                            new_self = new_self.grow();
                            displaced = new_self.reinsert(displaced);
                        }
                        return new_self;
                    }
                } else {
                // The key diverged from what we already have, so we need to introduce
                // a branch at the discriminating depth.

                    let sibling_leaf = Head::from(Leaf::new(branch_depth, key)).wrap_path(branch_depth, key);

                    let mut new_branch = Branch4::new(self.start_depth as usize, branch_depth, key);
                    new_branch.insert(sibling_leaf);
                    new_branch.insert(Head::<KEY_LEN>::from(self.clone()).wrap_path(branch_depth, key));
    
                    return Head::from(new_branch).wrap_path(self.start_depth as usize, key);
                }
            }

            fn with_start_depth(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let actual_start_depth = max(
                    new_start_depth as isize,
                    self.end_depth as isize - HEAD_FRAGMENT_LEN as isize,
                ) as usize;

                let mut new_fragment = [0; HEAD_FRAGMENT_LEN];
                for i in 0..new_fragment.len() {
                    let depth = actual_start_depth + i;
                    if(self.end_depth as usize <= depth) { break; }
                    new_fragment[i] = 
                        if depth < self.start_depth as usize {
                            println!("key @ {}", depth);
                            key[depth]
                        } else {
                            self.fragment[index_start(self.start_depth as usize, depth)]
                        }
                }
                Head::from(Self {
                    tag: HeadTag::$name,
                    start_depth: actual_start_depth as u8,
                    fragment: new_fragment,
                    end_depth: self.end_depth,
                    body: Arc::clone(&self.body),
                })
            }

            
        }
    };
}

create_branch!(Branch4, BranchBody4, ByteTable4);
create_branch!(Branch8, BranchBody8, ByteTable8);
create_branch!(Branch16, BranchBody16, ByteTable16);
create_branch!(Branch32, BranchBody32, ByteTable32);
create_branch!(Branch64, BranchBody64, ByteTable64);
create_branch!(Branch128, BranchBody128, ByteTable128);
create_branch!(Branch256, BranchBody256, ByteTable256);

macro_rules! create_grow {
    () => {};
    ($name:ident, $grown_name:ident, $grown_body_name:ident) => {
        impl<const KEY_LEN: usize> $name<KEY_LEN> {
            fn grow(&self) -> Head<KEY_LEN> {
                Head::<KEY_LEN>::from($grown_name {
                    tag: HeadTag::$grown_name,
                    start_depth: self.start_depth,
                    fragment: self.fragment,
                    end_depth: self.end_depth,
                    body: Arc::new($grown_body_name {
                        leaf_count: self.body.leaf_count,
                        //segment_count: self.segment_count,
                        //node_hash: self.node_hash,
                        child_set: self.body.child_set,
                        child_table: self.body.child_table.grow(),
                    }),
                })
            }
        }
    };
}

impl<const KEY_LEN: usize> Branch256<KEY_LEN> {
    fn grow(&self) -> Head<KEY_LEN> {
        panic!("`grow` called on Branch256");
    }
}

create_grow!(Branch4, Branch8, BranchBody8);
create_grow!(Branch8, Branch16, BranchBody16);
create_grow!(Branch16, Branch32, BranchBody32);
create_grow!(Branch32, Branch64, BranchBody64);
create_grow!(Branch64, Branch128, BranchBody128);
create_grow!(Branch128, Branch256, BranchBody256);

macro_rules! create_path {
    ($name:ident, $body_name:ident, $body_fragment_len:expr) => {
        #[derive(Clone, Debug)]
        #[repr(C)]
        pub struct $body_name<const KEY_LEN: usize> {
            child: Head<KEY_LEN>,
            //rc: AtomicU16,
            fragment: [u8; $body_fragment_len],
        }

        #[derive(Clone, Debug)]
        #[repr(C)]
        pub struct $name<const KEY_LEN: usize> {
            tag: HeadTag,
            start_depth: u8,
            fragment: [u8; HEAD_FRAGMENT_LEN],
            end_depth: u8,
            body: Arc<$body_name<KEY_LEN>>,
        }

        impl<const KEY_LEN: usize> From<$name<KEY_LEN>> for Head<KEY_LEN> {
            fn from(head: $name<KEY_LEN>) -> Self {
                unsafe {
                    transmute(head)
                }
            }
        }
        impl<const KEY_LEN: usize> $name<KEY_LEN> {
            fn new(start_depth: usize, key: &[u8; KEY_LEN], child: Head<KEY_LEN>) -> Self {
                let end_depth = child.start_depth();
                let mut body_fragment = [0; $body_fragment_len];
                copy_end(body_fragment.as_mut_slice(), &key[..], end_depth as usize);
    
                let path_body = Arc::new($body_name {
                    child: child,
                    //rc: AtomicU16::new(1),
                    fragment: body_fragment,
                });
    
                let actual_start_depth = max(
                    start_depth as isize,
                    end_depth as isize - ($body_fragment_len as isize + HEAD_FRAGMENT_LEN as isize),
                ) as usize;
    
                let mut fragment = [0; HEAD_FRAGMENT_LEN];
                copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);
    
                Self {
                    tag: HeadTag::$name,
                    start_depth: actual_start_depth as u8,
                    fragment: fragment,
                    end_depth: end_depth,
                    body: path_body,
                }
            }

            pub fn count(&self) -> u64 {
                self.body.child.count()
            }

            fn peek(&self, at_depth: usize) -> Option<u8> {
                if at_depth < self.start_depth as usize || self.end_depth as usize <= at_depth {
                    return None;
                }
                if at_depth < self.start_depth as usize + self.fragment.len() {
                    return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
                }
                return Some(
                    self.body.fragment[index_end(self.body.fragment.len(), self.end_depth as usize, at_depth)],
                );
            }
        
            fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
                result_set.unset_all();
                if at_depth == self.end_depth as usize {
                    result_set.set(self.body.child.peek(at_depth).expect("path child peek at child depth must succeed"));
                    return;
                }
            
                if let Some(byte_key) = self.peek(at_depth) {
                    result_set.set(byte_key);
                }
            }
        
            fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let mut branch_depth = self.start_depth as usize;
                while Some(key[branch_depth]) == self.peek(branch_depth) {
                    branch_depth += 1;
                }

                if branch_depth == self.end_depth as usize {
                // The entire fragment matched with the key.
                    let mut new_body = Arc::make_mut(&mut self.body);

                    let new_child = new_body.child.put(key);
                    if new_child.start_depth() != self.end_depth {
                        return new_child.wrap_path(self.start_depth as usize, key);
                    }

                    new_body.child = new_child;

                    return self.clone().into();
                } else {
                // The key diverged from what we already have, so we need to introduce
                // a branch at the discriminating depth.
                    let sibling_leaf = Head::<KEY_LEN>::from(Leaf::new(branch_depth, key)).wrap_path(branch_depth, key);

                    let mut new_branch = Branch4::new(self.start_depth as usize, branch_depth, key);

                    new_branch.insert(sibling_leaf);
                    new_branch.insert(Head::<KEY_LEN>::from(self.clone()).wrap_path(branch_depth, key));

                    return Head::<KEY_LEN>::from(new_branch).wrap_path(self.start_depth as usize, key);
                }
            }

            fn with_start_depth(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let actual_start_depth = max(
                    new_start_depth as isize,
                    self.end_depth as isize - (self.body.fragment.len() as isize + HEAD_FRAGMENT_LEN as isize),
                ) as usize;

                let head_end_depth = self.start_depth as usize + HEAD_FRAGMENT_LEN;

                let mut new_fragment = [0; HEAD_FRAGMENT_LEN];
                for i in 0..new_fragment.len() {
                    let depth = actual_start_depth + i;
                    if(self.end_depth as usize <= depth) { break; }
                    new_fragment[i] =
                        if(depth < self.start_depth as usize) {
                            key[depth]
                        } else {
                            if depth < head_end_depth {
                                self.fragment[index_start(self.start_depth as usize, depth)]}
                            else {
                                self.body.fragment[index_end(self.body.fragment.len(), self.end_depth as usize, depth)]
                            }
                        }
                }

                Head::<KEY_LEN>::from(Self {
                    tag: HeadTag::$name,
                    start_depth: actual_start_depth as u8,
                    fragment: new_fragment,
                    end_depth: self.end_depth,
                    body: Arc::clone(&self.body),
                })
            }

            fn insert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                panic!("`insert` called on path");
            }
        
            fn reinsert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
                panic!("`reinsert` called on path");
            }

            fn grow(&self) -> Head<KEY_LEN> {
                panic!("`grow` called on path");
            }
        }
    };
}

create_path!(Path14, PathBody14, 14);
create_path!(Path30, PathBody30, 30);
create_path!(Path46, PathBody46, 46);
create_path!(Path62, PathBody62, 62);

#[derive(Clone, Debug)]
pub struct Leaf<const KEY_LEN: usize> {
    tag: HeadTag,
    start_depth: u8,
    fragment: [u8; LEAF_FRAGMENT_LEN],
}

impl<const KEY_LEN: usize> From<Leaf<KEY_LEN>> for Head<KEY_LEN> {
    fn from(head: Leaf<KEY_LEN>) -> Self {
        unsafe {
            transmute(head)
        }
    }
}
impl<const KEY_LEN: usize> Leaf<KEY_LEN> {
    fn new(start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let actual_start_depth = max(
            start_depth,
            KEY_LEN - LEAF_FRAGMENT_LEN,
        );

        let mut fragment = [0; LEAF_FRAGMENT_LEN];

        copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

        Self {
            tag: HeadTag::Leaf,
            start_depth: actual_start_depth as u8,
            fragment: fragment,
        }
    }

    fn count(&self) -> u64 {
        1
    }

    fn peek(&self, at_depth: usize) -> Option<u8> {
        if KEY_LEN <= at_depth {
            return None;
        }
        return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }

    fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
        result_set.unset_all();
        if KEY_LEN <= at_depth {
            return;
        }
        result_set.set(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }

    fn put(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        let mut branch_depth = self.start_depth as usize;
        while Some(key[branch_depth]) == self.peek(branch_depth) {
            branch_depth += 1;
        }
        if branch_depth == KEY_LEN {
            return self.clone().into();
        } else {
            let sibling_leaf = Head::<KEY_LEN>::from(Leaf::new(branch_depth, key));

            let mut new_branch = Branch4::new(self.start_depth as usize, branch_depth, key);
            new_branch.insert(sibling_leaf);
            new_branch.insert(Head::<KEY_LEN>::from(self.clone()).wrap_path(branch_depth, key));
    
            return Head::<KEY_LEN>::from(new_branch).wrap_path(self.start_depth as usize, key);
        }
    }

    fn with_start_depth(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        assert!(new_start_depth <= KEY_LEN);

        let actual_start_depth = max(
            new_start_depth as isize,
            KEY_LEN as isize - ( LEAF_FRAGMENT_LEN as isize ),
        ) as usize;

        let mut new_fragment = [0; LEAF_FRAGMENT_LEN];
        for i in 0..new_fragment.len() {
            let depth = actual_start_depth + i;
            if KEY_LEN <= depth { break; }
            new_fragment[i] = 
                if depth < self.start_depth as usize {
                    key[depth]
                } else {
                    self.fragment[index_start(self.start_depth as usize, depth)]
                }
        }

        Head::<KEY_LEN>::from(Self {
            tag: HeadTag::Leaf,
            start_depth: actual_start_depth as u8,
            fragment: new_fragment,
        })
    }

    fn insert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`insert` called on leaf");
    }

    fn reinsert(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`reinsert` called on leaf");
    }

    fn grow(&self) -> Head<KEY_LEN> {
        panic!("`grow` called on leaf");
    }
}
/*
macro_rules! dispatch {
    ($self:ident, $name:ident, $call:tt) => {
        unsafe {
            match $self.unknown.tag {
                HeadTag::Empty => {
                    let $name = $self.empty;
                    $call
                },
                HeadTag::Leaf => {
                    let $name = $self.leaf;
                    $call
                },
                HeadTag::Path14 => {
                    let $name = $self.path14;
                    $call
                },
                HeadTag::Path30 => {
                    let $name = $self.path30;
                    $call
                },
                HeadTag::Path46 => {
                    let $name = $self.path46;
                    $call
                },
                HeadTag::Path62 => {
                    let $name = $self.path62;
                    $call
                },
                HeadTag::Branch4 => {
                    let $name = $self.branch4;
                    $call
                },
                HeadTag::Branch8 => {
                    let $name = $self.branch8;
                    $call
                },
                HeadTag::Branch16 => {
                    let $name = $self.branch16;
                    $call
                },
                HeadTag::Branch32 => {
                    let $name = $self.branch32;
                    $call
                },
                HeadTag::Branch64 => {
                    let $name = $self.branch64;
                    $call
                },
                HeadTag::Branch128 => {
                    let $name = $self.branch128;
                    $call
                },
                HeadTag::Branch256 => {
                    let $name = $self.branch256;
                    $call
                },
            }
        }
    };
}
*/

#[derive(Debug)]
#[repr(C)]
struct Unknown {
    tag: HeadTag,
    start_depth: u8,
    key: u8,
    ignore: [MaybeUninit<u8>; 13],
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct Empty {
    tag: HeadTag,
    ignore: [MaybeUninit<u8>; 15],
}

impl<const KEY_LEN: usize> From<Empty> for Head<KEY_LEN> {
    fn from(head: Empty) -> Self {
        unsafe {
            transmute(head)
        }
    }
}
impl Empty {
    fn new() -> Self {
        Self {
            tag: HeadTag::Empty,
            ignore: MaybeUninit::uninit_array()
        }
    }

    fn count(&self) -> u64 {
        0
    }

    fn with_start_depth<const KEY_LEN: usize>(&self, _new_start_depth: usize, _key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        panic!("`with_start_depth` called on empty");
    }

    fn peek(&self, _at_depth: usize) -> Option<u8> {
        None
    }
    
    fn propose(&self, _at_depth: usize, result_set: &mut ByteBitset) {
        result_set.unset_all();
    }

    fn put<const KEY_LEN: usize>(&mut self, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        Head::<KEY_LEN>::from(Leaf::new(0, key)).wrap_path(0, key)
    }

    fn insert<const KEY_LEN: usize>(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`insert` called on empty");
    }

    fn reinsert<const KEY_LEN: usize>(&mut self, _child: Head<KEY_LEN>) -> Head<KEY_LEN> {
        panic!("`reinsert` called on empty");
    }

    fn grow<const KEY_LEN: usize>(&self) -> Head<KEY_LEN> {
        panic!("`grow` called on empty");
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

#[repr(C)]
pub union Head<const KEY_LEN: usize> {
    unknown: ManuallyDrop<Unknown>,
    empty: ManuallyDrop<Empty>,
    branch4: ManuallyDrop<Branch4<KEY_LEN>>,
    branch8: ManuallyDrop<Branch8<KEY_LEN>>,
    branch16: ManuallyDrop<Branch16<KEY_LEN>>,
    branch32: ManuallyDrop<Branch32<KEY_LEN>>,
    branch64: ManuallyDrop<Branch64<KEY_LEN>>,
    branch128: ManuallyDrop<Branch128<KEY_LEN>>,
    branch256: ManuallyDrop<Branch256<KEY_LEN>>,
    path14: ManuallyDrop<Path14<KEY_LEN>>,
    path30: ManuallyDrop<Path30<KEY_LEN>>,
    path46: ManuallyDrop<Path46<KEY_LEN>>,
    path62: ManuallyDrop<Path62<KEY_LEN>>,
    leaf: ManuallyDrop<Leaf<KEY_LEN>>,
}

unsafe impl<const KEY_LEN: usize> ByteEntry for Head<KEY_LEN> {
    fn zeroed() -> Self {
        Empty::new().into()
    }

    fn key(&self) -> Option<u8> {
        unsafe {
            if self.unknown.tag == HeadTag::Empty {
                None
            } else {
                Some(self.unknown.key)
            }
        }
    }
}

impl<const KEY_LEN: usize> fmt::Debug for Head<KEY_LEN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.fmt(f),
                HeadTag::Leaf => self.leaf.fmt(f),
                HeadTag::Path14 => self.path14.fmt(f),
                HeadTag::Path30 => self.path30.fmt(f),
                HeadTag::Path46 => self.path46.fmt(f),
                HeadTag::Path62 => self.path62.fmt(f),
                HeadTag::Branch4 => self.branch4.fmt(f),
                HeadTag::Branch8 => self.branch8.fmt(f),
                HeadTag::Branch16 => self.branch16.fmt(f),
                HeadTag::Branch32 => self.branch32.fmt(f),
                HeadTag::Branch64 => self.branch64.fmt(f),
                HeadTag::Branch128 => self.branch128.fmt(f),
                HeadTag::Branch256 => self.branch256.fmt(f),
            }
        }
    }
}

impl<const KEY_LEN: usize> Clone for Head<KEY_LEN> {
    fn clone(&self) -> Self {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => Self { empty: self.empty.clone() },
                HeadTag::Leaf => Self { leaf: self.leaf.clone() },
                HeadTag::Path14 => Self { path14: self.path14.clone() },
                HeadTag::Path30 => Self { path30: self.path30.clone() },
                HeadTag::Path46 => Self { path46: self.path46.clone() },
                HeadTag::Path62 => Self { path62: self.path62.clone() },
                HeadTag::Branch4 => Self { branch4: self.branch4.clone() },
                HeadTag::Branch8 => Self { branch8: self.branch8.clone() },
                HeadTag::Branch16 => Self { branch16: self.branch16.clone() },
                HeadTag::Branch32 => Self { branch32: self.branch32.clone() },
                HeadTag::Branch64 => Self { branch64: self.branch64.clone() },
                HeadTag::Branch128 => Self { branch128: self.branch128.clone() },
                HeadTag::Branch256 => Self { branch256: self.branch256.clone() },
            }
        }
    }
}

impl<const KEY_LEN: usize> Drop for Head<KEY_LEN> {
    fn drop(&mut self) {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => ManuallyDrop::drop(&mut self.empty),
                HeadTag::Leaf => ManuallyDrop::drop(&mut self.leaf),
                HeadTag::Path14 => ManuallyDrop::drop(&mut self.path14),
                HeadTag::Path30 => ManuallyDrop::drop(&mut self.path30),
                HeadTag::Path46 => ManuallyDrop::drop(&mut self.path46),
                HeadTag::Path62 => ManuallyDrop::drop(&mut self.path62),
                HeadTag::Branch4 => ManuallyDrop::drop(&mut self.branch4),
                HeadTag::Branch8 => ManuallyDrop::drop(&mut self.branch8),
                HeadTag::Branch16 => ManuallyDrop::drop(&mut self.branch16),
                HeadTag::Branch32 => ManuallyDrop::drop(&mut self.branch32),
                HeadTag::Branch64 => ManuallyDrop::drop(&mut self.branch64),
                HeadTag::Branch128 => ManuallyDrop::drop(&mut self.branch128),
                HeadTag::Branch256 => ManuallyDrop::drop(&mut self.branch256),
            }
        }
    }
}

impl<const KEY_LEN: usize> Default for Head<KEY_LEN> {
    fn default() -> Self {
        Empty::new().into()
    }
}

impl<const KEY_LEN: usize> Head<KEY_LEN> {
    fn wrap_path(&self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let expanded = self.with_start_depth(start_depth, key);

        let actual_start_depth = expanded.start_depth() as usize;
        if start_depth == actual_start_depth {
            return expanded;
        }

        let path_length = actual_start_depth - start_depth;

        if path_length <= 14 + HEAD_FRAGMENT_LEN {
            return Path14::new(start_depth, &key, expanded).into();
        }

        if path_length <= 30 + HEAD_FRAGMENT_LEN {
            return Path30::new(start_depth, &key, expanded).into();
        }

        if path_length <= 46 + HEAD_FRAGMENT_LEN {
            return Path46::new(start_depth, &key, expanded).into();
        }

        if path_length <= 62 + HEAD_FRAGMENT_LEN {
            return Path62::new(start_depth, &key, expanded).into();
        }

        panic!("Fragment too long for path to hold.");
    }

    fn start_depth(&self) -> u8 {
        unsafe {
            if self.unknown.tag == HeadTag::Empty {
                panic!("Called `start_depth` on `Empty`.");
            }
            self.unknown.start_depth
        }
    }

    fn count(&self) -> u64 {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.count(),
                HeadTag::Leaf => self.leaf.count(),
                HeadTag::Path14 => self.path14.count(),
                HeadTag::Path30 => self.path30.count(),
                HeadTag::Path46 => self.path46.count(),
                HeadTag::Path62 => self.path62.count(),
                HeadTag::Branch4 => self.branch4.count(),
                HeadTag::Branch8 => self.branch8.count(),
                HeadTag::Branch16 => self.branch16.count(),
                HeadTag::Branch32 => self.branch32.count(),
                HeadTag::Branch64 => self.branch64.count(),
                HeadTag::Branch128 => self.branch128.count(),
                HeadTag::Branch256 => self.branch256.count(),
            }
        }
    }

    fn with_start_depth(&self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.with_start_depth(new_start_depth, key),
                HeadTag::Leaf => self.leaf.with_start_depth(new_start_depth, key),
                HeadTag::Path14 => self.path14.with_start_depth(new_start_depth, key),
                HeadTag::Path30 => self.path30.with_start_depth(new_start_depth, key),
                HeadTag::Path46 => self.path46.with_start_depth(new_start_depth, key),
                HeadTag::Path62 => self.path62.with_start_depth(new_start_depth, key),
                HeadTag::Branch4 => self.branch4.with_start_depth(new_start_depth, key),
                HeadTag::Branch8 => self.branch8.with_start_depth(new_start_depth, key),
                HeadTag::Branch16 => self.branch16.with_start_depth(new_start_depth, key),
                HeadTag::Branch32 => self.branch32.with_start_depth(new_start_depth, key),
                HeadTag::Branch64 => self.branch64.with_start_depth(new_start_depth, key),
                HeadTag::Branch128 => self.branch128.with_start_depth(new_start_depth, key),
                HeadTag::Branch256 => self.branch256.with_start_depth(new_start_depth, key),
            }
        }
    }

    fn peek(&self, at_depth: usize) -> Option<u8> {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.peek(at_depth),
                HeadTag::Leaf => self.leaf.peek(at_depth),
                HeadTag::Path14 => self.path14.peek(at_depth),
                HeadTag::Path30 => self.path30.peek(at_depth),
                HeadTag::Path46 => self.path46.peek(at_depth),
                HeadTag::Path62 => self.path62.peek(at_depth),
                HeadTag::Branch4 => self.branch4.peek(at_depth),
                HeadTag::Branch8 => self.branch8.peek(at_depth),
                HeadTag::Branch16 => self.branch16.peek(at_depth),
                HeadTag::Branch32 => self.branch32.peek(at_depth),
                HeadTag::Branch64 => self.branch64.peek(at_depth),
                HeadTag::Branch128 => self.branch128.peek(at_depth),
                HeadTag::Branch256 => self.branch256.peek(at_depth),
            }
        }
    }

    fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.propose(at_depth, result_set),
                HeadTag::Leaf => self.leaf.propose(at_depth, result_set),
                HeadTag::Path14 => self.path14.propose(at_depth, result_set),
                HeadTag::Path30 => self.path30.propose(at_depth, result_set),
                HeadTag::Path46 => self.path46.propose(at_depth, result_set),
                HeadTag::Path62 => self.path62.propose(at_depth, result_set),
                HeadTag::Branch4 => self.branch4.propose(at_depth, result_set),
                HeadTag::Branch8 => self.branch8.propose(at_depth, result_set),
                HeadTag::Branch16 => self.branch16.propose(at_depth, result_set),
                HeadTag::Branch32 => self.branch32.propose(at_depth, result_set),
                HeadTag::Branch64 => self.branch64.propose(at_depth, result_set),
                HeadTag::Branch128 => self.branch128.propose(at_depth, result_set),
                HeadTag::Branch256 => self.branch256.propose(at_depth, result_set),
            }
        }
    }

    fn put(&mut self, key: &[u8; KEY_LEN]) -> Self {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.put(key),
                HeadTag::Leaf => self.leaf.put(key),
                HeadTag::Path14 => self.path14.put(key),
                HeadTag::Path30 => self.path30.put(key),
                HeadTag::Path46 => self.path46.put(key),
                HeadTag::Path62 => self.path62.put(key),
                HeadTag::Branch4 => self.branch4.put(key),
                HeadTag::Branch8 => self.branch8.put(key),
                HeadTag::Branch16 => self.branch16.put(key),
                HeadTag::Branch32 => self.branch32.put(key),
                HeadTag::Branch64 => self.branch64.put(key),
                HeadTag::Branch128 => self.branch128.put(key),
                HeadTag::Branch256 => self.branch256.put(key),
            }
        } 
    }

    fn insert(&mut self, child: Self) -> Self {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.insert(child),
                HeadTag::Leaf => self.leaf.insert(child),
                HeadTag::Path14 => self.path14.insert(child),
                HeadTag::Path30 => self.path30.insert(child),
                HeadTag::Path46 => self.path46.insert(child),
                HeadTag::Path62 => self.path62.insert(child),
                HeadTag::Branch4 => self.branch4.insert(child),
                HeadTag::Branch8 => self.branch8.insert(child),
                HeadTag::Branch16 => self.branch16.insert(child),
                HeadTag::Branch32 => self.branch32.insert(child),
                HeadTag::Branch64 => self.branch64.insert(child),
                HeadTag::Branch128 => self.branch128.insert(child),
                HeadTag::Branch256 => self.branch256.insert(child),
            }
        } 
    }

    fn reinsert(&mut self, child: Self) -> Self {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.reinsert(child),
                HeadTag::Leaf => self.leaf.reinsert(child),
                HeadTag::Path14 => self.path14.reinsert(child),
                HeadTag::Path30 => self.path30.reinsert(child),
                HeadTag::Path46 => self.path46.reinsert(child),
                HeadTag::Path62 => self.path62.reinsert(child),
                HeadTag::Branch4 => self.branch4.reinsert(child),
                HeadTag::Branch8 => self.branch8.reinsert(child),
                HeadTag::Branch16 => self.branch16.reinsert(child),
                HeadTag::Branch32 => self.branch32.reinsert(child),
                HeadTag::Branch64 => self.branch64.reinsert(child),
                HeadTag::Branch128 => self.branch128.reinsert(child),
                HeadTag::Branch256 => self.branch256.reinsert(child),
            }
        } 
    }

    fn grow(&self) -> Self {
        unsafe {
            match self.unknown.tag {
                HeadTag::Empty => self.empty.grow(),
                HeadTag::Leaf => self.leaf.grow(),
                HeadTag::Path14 => self.path14.grow(),
                HeadTag::Path30 => self.path30.grow(),
                HeadTag::Path46 => self.path46.grow(),
                HeadTag::Path62 => self.path62.grow(),
                HeadTag::Branch4 => self.branch4.grow(),
                HeadTag::Branch8 => self.branch8.grow(),
                HeadTag::Branch16 => self.branch16.grow(),
                HeadTag::Branch32 => self.branch32.grow(),
                HeadTag::Branch64 => self.branch64.grow(),
                HeadTag::Branch128 => self.branch128.grow(),
                HeadTag::Branch256 => self.branch256.grow(),
            }
        } 
    }

    /*
    fn hash(&self, prefix: [u8; KEY_LEN]) -> u128 {

/* Path
                    var key = prefix;

                    var i = self.start_depth;
                    while(i < self.branch_depth):(i += 1) {
                        key[i] = self.peek(i).?;
                    }

                    return self.body.child.hash(key);
*/

/* Leaf

*/

        match self {
            Self::Empty { .. } => panic!("Called `hash` on `Empty`."),
            Self::Leaf { start_depth, fragment, .. } => {
                let mut key = prefix;

                let start = *start_depth as usize;
                for i in start..KEY_LEN
                    key[i] = fragment[index_start(start, i)];
                }

                let mut hasher = SipHasher24::new_with_key(&SIP_KEY);

                return Hash.init(&key);
            },
            Self::Path14 { body, .. } => body.child.hash(),
            Self::Path30 { body, .. } => body.child.hash(),
            Self::Path46 { body, .. } => body.child.hash(),
            Self::Path62 { body, .. } => body.child.hash(),
            Self::Branch4 { body, .. } => body.hash,
            Self::Branch8 { body, .. } => body.hash,
            Self::Branch16 { body, .. } => body.hash,
            Self::Branch32 { body, .. } => body.hash,
            Self::Branch64 { body, .. } => body.hash,
            Self::Branch128 { body, .. } => body.hash,
            Self::Branch256 { body, .. } => body.hash,
        }
    }
    */
    /*
    fn get_branch(at_depth: usize, byte_key: u8, start_depth: usize, end_depth: usize, head_fragment: &[u8], child_set: &ByteBitset, head: &Self) {
        if at_depth == end_depth {
            if child_set.is_set(byte_key) {
                return ;
            } else {
                return ;
            }
        }
        if let Some(byte_key) = Self::peek_branch(at_depth, start_depth, end_depth, head_fragment) {
            return head;
        }
    }

    fn propose_path(at_depth: usize, start_depth: usize, end_depth: usize, head_fragment: &[u8], body_fragment: &[u8], child: &Head<KEY_LEN>, result_set: &mut ByteBitset) {
        result_set.unset_all();
        if at_depth == end_depth {
            result_set.set(child.peek(at_depth).expect("path child peek at child depth must succeed"));
            return;
        }
    
        if let Some(byte_key) = Self::peek_path(at_depth, start_depth, end_depth, head_fragment, body_fragment) {
            result_set.set(byte_key);
        }
    }

    fn get(&self, at_depth: usize, byte_key: u8) {
        match self {
            Self::Empty { .. } => panic!("Called `propose` on `Empty`."),
            Self::Leaf {
                fragment,
                start_depth,
                ..
            } => {
                result_set.unset_all();
                if KEY_LEN <= at_depth {
                    return; //TODO: do we need this vs. assert?
                }
                result_set.set(fragment[index_start(*start_depth as usize, at_depth)]);
            }
            Self::Path14 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => Self::propose_path(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.fragment[..], &body.child, result_set),
            Self::Path30 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => Self::propose_path(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.fragment[..], &body.child, result_set),
            Self::Path46 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => Self::propose_path(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.fragment[..], &body.child, result_set),
            Self::Path62 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => Self::propose_path(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.fragment[..], &body.child, result_set),
            Self::Branch4 {
                start_depth,
                end_depth,
                fragment,
                body
            } => Self::propose_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.child_set, result_set),
            Self::Branch8 {
                start_depth,
                end_depth,
                fragment,
                body
            } => Self::propose_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.child_set, result_set),
            Self::Branch16 {
                start_depth,
                end_depth,
                fragment,
                body
            } => Self::propose_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.child_set, result_set),
            Self::Branch32 {
                start_depth,
                end_depth,
                fragment,
                body
            } => Self::propose_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.child_set, result_set),
            Self::Branch64 {
                start_depth,
                end_depth,
                fragment,
                body
            } => Self::propose_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.child_set, result_set),
            Self::Branch128 {
                start_depth,
                end_depth,
                fragment,
                body
            } => Self::propose_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.child_set, result_set),
            Self::Branch256 {
                start_depth,
                end_depth,
                fragment,
                body
            } => Self::propose_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..], &body.child_set, result_set),
        }
    }

    */
}

#[derive(Debug, Clone)]
pub struct PACT<const KEY_LEN: usize> {
    root: Head<KEY_LEN>,
}

impl<'a, const KEY_LEN: usize> PACT<KEY_LEN>
where
    [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1]: Sized,
{
    const KEY_LEN_CHECK: usize = KEY_LEN - 64;

    pub fn new() -> Self {
        PACT {
            root: Empty::new().into(),
        }
    }

    pub fn put(&mut self, key: [u8; KEY_LEN]) {
        self.root = self.root.put(&key);
    }

    pub fn len(&self) -> u64 {
        self.root.count()
    }

    pub fn cursor(&self) -> PACTCursor<KEY_LEN> {
        return PACTCursor::new(self);
    }
}

pub struct PACTCursor<'a, const KEY_LEN: usize>
where
    [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1]: Sized,
{
    depth: usize,
    path: [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1],
}

impl<'a, const KEY_LEN: usize> PACTCursor<'a, KEY_LEN>
where
    [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1]: Sized,
{
    pub fn new(tree: &'a PACT<KEY_LEN>) -> Self {
        let mut new = Self {
            depth: 0,
            path: [None; KEY_LEN + 1]
        };
        new.path[0] = Some(&tree.root);
        return new;
    }
}

impl<'a, const KEY_LEN: usize> ByteCursor for PACTCursor<'a, KEY_LEN>
where
    [Option<&'a Head<KEY_LEN>>; KEY_LEN + 1]: Sized,
{
    fn peek(&self) -> Option<u8> {
        return self.path[self.depth]
                .expect("peeked path should exist")
                .peek(self.depth);
    }

    fn propose(&self, bitset: &mut ByteBitset) {
        self.path[self.depth]
        .expect("proposed path should exist")
        .propose(self.depth, bitset);
    }

    fn pop(&mut self) {
        self.depth -= 1;
    }

    fn push(&mut self, byte: u8) {
        self.path[self.depth + 1] = self.path[self.depth];
        //.expect("pushed path should exist")
        //.get(self.depth, byte);
        self.depth += 1;
    }

    fn segment_count(&self) -> u32 {
        return 0;
        //return self.path[self.depth].segment_count(self.depth);
    }
}

/*
    pub fn iterate(self: Cursor) CursorIterator(Cursor, key_length) {
        return CursorIterator(Cursor, key_length).init(self);
    }
*/

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use itertools::Itertools;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head<64>>(), 16);
    }

    #[test]
    fn empty_tree() {
        init();
        
        let _tree = PACT::<64>::new();
    }

    #[test]
    fn tree_put_one() {
        init();

        const KEY_SIZE: usize = 64;
        let mut tree = PACT::<KEY_SIZE>::new();
        let key = [0; KEY_SIZE];
        tree.put(key);
    }

    #[test]
    fn branch_size() {
        assert_eq!(mem::size_of::<ByteTable4<Head<64>>>(), 64);
        assert_eq!(mem::size_of::<BranchBody4<64>>(), 64 * 2);
        assert_eq!(mem::size_of::<BranchBody8<64>>(), 64 * 3);
        assert_eq!(mem::size_of::<BranchBody16<64>>(), 64 * 5);
        assert_eq!(mem::size_of::<BranchBody32<64>>(), 64 * 9);
        assert_eq!(mem::size_of::<BranchBody64<64>>(), 64 * 17);
        assert_eq!(mem::size_of::<BranchBody128<64>>(), 64 * 33);
        assert_eq!(mem::size_of::<BranchBody256<64>>(), 64 * 65);
    }

    #[test]
    fn fragment_size() {
        assert_eq!(mem::size_of::<PathBody14<64>>(), 16 * 2);
        assert_eq!(mem::size_of::<PathBody30<64>>(), 16 * 3);
        assert_eq!(mem::size_of::<PathBody46<64>>(), 16 * 4);
        assert_eq!(mem::size_of::<PathBody62<64>>(), 16 * 5);
    }

    
    proptest! {
        #[test]
        fn tree_put(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64>::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(key);
            }
            //let entry_set = HashSet::from_iter(entries.iter().cloned());
        }

        #[test]
        fn tree_len(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut tree = PACT::<64>::new();
            let mut set = HashSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(key);
                set.insert(key);
            }
            prop_assert_eq!(set.len() as u64, tree.len())
        }
    }
}
