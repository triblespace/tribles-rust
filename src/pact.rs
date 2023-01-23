use crate::bitset::ByteBitset;
use crate::bytetable;
use crate::bytetable::*;
use crate::query::{ ByteCursor, CursorIterator };
use std::sync::Once;
use std::cmp::{max, min};
use std::mem;
use std::sync::Arc;
use rand::thread_rng;
use rand::RngCore;
use core::hash::Hasher;
use siphasher::sip128::{Hasher128, SipHasher24};

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

macro_rules! create_grow {
    () => {};
    ($grown_name:ident) => {
        fn grow(self) -> $grown_name<KEY_LEN> {
            $grown_name {
                leaf_count: self.leaf_count,
                //segment_count: self.segment_count,
                //node_hash: self.node_hash,
                child_set: self.child_set,
                child_table: self.child_table.grow(),
            }
        }
    };
}

macro_rules! create_branchbody {
    ($name:ident, $table:tt, $($grown_name:ident)?) => {
        #[derive(Clone)]
        #[repr(C)]
        pub struct $name<const KEY_LEN: usize> {
            leaf_count: u64,
            //rc: AtomicU16,
            //segment_count: u32, //TODO: increase this to a u48
            //hash: u128,
            child_set: ByteBitset,
            child_table: $table<Head<KEY_LEN>>,
        }

        impl<const KEY_LEN: usize> $name<KEY_LEN> {
            create_grow!($($grown_name)?);
        }
    };
}

create_branchbody!(BranchBody4, ByteTable4, BranchBody8);
create_branchbody!(BranchBody8, ByteTable8, BranchBody16);
create_branchbody!(BranchBody16, ByteTable16, BranchBody32);
create_branchbody!(BranchBody32, ByteTable32, BranchBody64);
create_branchbody!(BranchBody64, ByteTable64, BranchBody128);
create_branchbody!(BranchBody128, ByteTable128, BranchBody256);
create_branchbody!(BranchBody256, ByteTable256,);

macro_rules! create_path {
    ($name:ident, $body_name:ident, $body_fragment_len:expr) => {
        #[derive(Clone)]
        #[repr(C)]
        pub struct $body_name<const KEY_LEN: usize> {
            child: Head<KEY_LEN>,
            //rc: AtomicU16,
            fragment: [u8; $body_fragment_len],
        }

        #[derive(Clone)]
        pub struct $name<const KEY_LEN: usize> {
            start_depth: u8,
            fragment: [u8; HEAD_FRAGMENT_LEN],
            end_depth: u8,
            body: Arc<$body_name<KEY_LEN>>,
        }

        impl<const KEY_LEN: usize> $name<KEY_LEN> {
            fn new(start_depth: usize, key: &[u8; KEY_LEN], child: Head<KEY_LEN>) -> Head<KEY_LEN> {
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
    
                Head::$name {
                    head: Self {
                        start_depth: actual_start_depth as u8,
                        fragment: fragment,
                        end_depth: end_depth,
                        body: path_body,
                    }
                }
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
        
            fn put(self, at_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
                let mut branch_depth = at_start_depth;

                let head_end_depth = self.start_depth as usize + HEAD_FRAGMENT_LEN;
                while branch_depth < self.end_depth as usize {
                    let prefix_matches = if branch_depth < head_end_depth {
                        key[branch_depth] == self.fragment[index_start(self.start_depth as usize, branch_depth)]}
                    else {
                        key[branch_depth] == self.body.fragment[index_end(self.body.fragment.len(), self.end_depth as usize, branch_depth)]
                    };
                    if prefix_matches{
                        branch_depth += 1
                    } else {
                        break;
                    }
                }

                let mut new_body = Arc::try_unwrap(self.body).unwrap_or_else(|arc| (*arc).clone());

                if branch_depth == self.end_depth as usize {
                    // The entire infix matched with the key, i.e. branch_depth == self.branch_depth.
                    let new_child = new_body.child.put(
                        self.end_depth as usize,
                        key
                    );
                    if new_child.start_depth() != self.end_depth {
                        return new_child.wrap_path(at_start_depth, key);
                    }

                    new_body.child = new_child;

                    return Head::$name {
                        head: Self {
                            start_depth: self.start_depth,
                            end_depth: self.end_depth,
                            fragment: self.fragment,
                            body: Arc::new(new_body),
                        }
                    };
                }

                let sibling_leaf_node =
                    Head::newLeaf(branch_depth, key).wrap_path(branch_depth, key);

                let self_node = Head::$name {
                    head: Self {
                        start_depth: self.start_depth,
                        end_depth: self.end_depth,
                        fragment: self.fragment,
                        body: Arc::new(new_body)
                    }
                };

                let branch_head =
                    Head::newBranch(at_start_depth, branch_depth, key, sibling_leaf_node, self_node.wrap_path(branch_depth, key));

                return branch_head.wrap_path(at_start_depth, key);
            }

            fn with_start_depth(self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
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

                Head::$name {
                    head: Self {
                        start_depth: actual_start_depth as u8,
                        fragment: new_fragment,
                        end_depth: self.end_depth,
                        body: self.body,
                    }
                }
            }
        }
    };
}

create_path!(Path14, PathBody14, 14);
create_path!(Path30, PathBody30, 30);
create_path!(Path46, PathBody46, 46);
create_path!(Path62, PathBody62, 62);

#[derive(Clone)]
pub struct LeafHead<const KEY_LEN: usize> {
    start_depth: u8,
    fragment: [u8; LEAF_FRAGMENT_LEN],
}

impl<const KEY_LEN: usize> LeafHead<KEY_LEN> {
    fn peek(&self, at_depth: usize) -> Option<u8> {
        if KEY_LEN <= at_depth {
            return None; //TODO: do we need this vs. assert?
        }
        return Some(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }

    fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
        result_set.unset_all();
        if KEY_LEN <= at_depth {
            return; //TODO: do we need this vs. assert?
        }
        result_set.set(self.fragment[index_start(self.start_depth as usize, at_depth)]);
    }

    fn put(self, at_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        let mut branch_depth = at_start_depth;
        while branch_depth < KEY_LEN {
            if key[branch_depth]
                == self.fragment[index_start(self.start_depth as usize, branch_depth)]
            {
                branch_depth += 1
            } else {
                break;
            }
        }
        if branch_depth == KEY_LEN {
            return Head::Leaf { head: self };
        }

        let sibling_leaf_node = Head::newLeaf(branch_depth, key);

        let branch_head = Head::newBranch(
            at_start_depth,
            branch_depth,
            key,
            sibling_leaf_node,
            self.with_start_depth(branch_depth, key),
        );

        return branch_head.wrap_path(at_start_depth, key);
    }

    fn with_start_depth(self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
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

        Head::Leaf {
            head: Self {
                start_depth: actual_start_depth as u8,
                fragment: new_fragment,
            }
        }
    }
}

#[derive(Clone)]
pub enum Head<const KEY_LEN: usize> {
    // MUST be first so that discriminant = 0;
    Empty {padding: [u8; 15]},
    Leaf {head: LeafHead<KEY_LEN>},
    Path14 {head: Path14<KEY_LEN>},
    Path30 {head: Path30<KEY_LEN>},
    Path46 {head: Path46<KEY_LEN>},
    Path62 {head: Path62<KEY_LEN>},
    Branch4 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: Arc<BranchBody4<KEY_LEN>>,
    },
    Branch8 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: Arc<BranchBody8<KEY_LEN>>,
    },
    Branch16 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: Arc<BranchBody16<KEY_LEN>>,
    },
    Branch32 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: Arc<BranchBody32<KEY_LEN>>,
    },
    Branch64 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: Arc<BranchBody64<KEY_LEN>>,
    },
    Branch128 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: Arc<BranchBody128<KEY_LEN>>,
    },
    Branch256 {
        start_depth: u8,
        fragment: [u8; HEAD_FRAGMENT_LEN],
        end_depth: u8,
        body: Arc<BranchBody256<KEY_LEN>>,
    },
}

impl<const KEY_LEN: usize> Head<KEY_LEN> {
    fn newEmpty() -> Head<KEY_LEN> {
        Self::Empty { padding: [0; 15] }
    }

    fn newLeaf(start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let actual_start_depth = max(
            start_depth,
            KEY_LEN - LEAF_FRAGMENT_LEN,
        );

        let mut fragment = [0; LEAF_FRAGMENT_LEN];

        copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

        Self::Leaf {
            head: LeafHead {
                start_depth: actual_start_depth as u8,
                fragment: fragment,
            }
        }
    }

    fn newBranch(
        start_depth: usize,
        end_depth: usize,
        key: &[u8; KEY_LEN],
        left: Head<KEY_LEN>,
        right: Head<KEY_LEN>,
    ) -> Head<KEY_LEN> {
        /*
        let layout = Layout::new::<$body_name<KEY_LEN>>();
        let branch_body = Global
        .allocate_zeroed(layout)
        .unwrap()
        .cast::<$body_name<KEY_LEN>>();
        *(branch_body.as_mut()) = $body_name {
            leaf_count: 0,
            rc: AtomicU16::new(1),
            segment_count: 0,
            node_hash: 0,
            child_set: ByteBitset::new_empty(),
            child_table: $table::new(),
        };
        */

        let mut branch_body = BranchBody4 {
            leaf_count: left.count() + right.count(),
            //rc: AtomicU16::new(1),
            //segment_count: 0,
            //node_hash: 0,
            child_set: ByteBitset::new_empty(),
            child_table: ByteTable4::new(),
        };

        branch_body.child_set.set(left.key().unwrap());
        branch_body.child_set.set(right.key().unwrap());
        branch_body.child_table.put(left);
        branch_body.child_table.put(right);

        let actual_start_depth = max(
            start_depth as isize,
            end_depth as isize - HEAD_FRAGMENT_LEN as isize,
        ) as usize;

        let mut fragment = [0; HEAD_FRAGMENT_LEN];
        copy_start(fragment.as_mut_slice(), &key[..], actual_start_depth);

        Self::Branch4 {
            start_depth: actual_start_depth as u8,
            fragment: fragment,
            end_depth: end_depth as u8,
            body: Arc::new(branch_body),
        }
    }

    fn wrap_path(self, start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        let expanded = self.with_start_depth(start_depth, key);

        let actual_start_depth = expanded.start_depth() as usize;
        if start_depth == actual_start_depth {
            return expanded;
        }

        let path_length = actual_start_depth - start_depth;

        if path_length <= 14 + HEAD_FRAGMENT_LEN {
            return Path14::new(start_depth, &key, expanded);
        }

        if path_length <= 30 + HEAD_FRAGMENT_LEN {
            return Path30::new(start_depth, &key, expanded);
        }

        if path_length <= 46 + HEAD_FRAGMENT_LEN {
            return Path46::new(start_depth, &key, expanded);
        }

        if path_length <= 62 + HEAD_FRAGMENT_LEN {
            return Path62::new(start_depth, &key, expanded);
        }

        panic!("Fragment too long for path to hold.");
    }

    fn start_depth(&self) -> u8 {
        match self {
            Self::Empty { .. } => panic!("Called `start_depth` on `Empty`."),
            Self::Leaf { head } => head.start_depth,
            Self::Path14 { head } => head.start_depth,
            Self::Path30 { head } => head.start_depth,
            Self::Path46 { head } => head.start_depth,
            Self::Path62 { head } => head.start_depth,
            Self::Branch4 { start_depth, .. } => *start_depth,
            Self::Branch8 { start_depth, .. } => *start_depth,
            Self::Branch16 { start_depth, .. } => *start_depth,
            Self::Branch32 { start_depth, .. } => *start_depth,
            Self::Branch64 { start_depth, .. } => *start_depth,
            Self::Branch128 { start_depth, .. } => *start_depth,
            Self::Branch256 { start_depth, .. } => *start_depth,
        }
    }

    fn count(&self) -> u64 {
        match self {
            Self::Empty { .. } => 0,
            Self::Leaf { .. } => 1,
            Self::Path14 { head, .. } => head.body.child.count(),
            Self::Path30 { head, .. } => head.body.child.count(),
            Self::Path46 { head, .. } => head.body.child.count(),
            Self::Path62 { head, .. } => head.body.child.count(),
            Self::Branch4 { body, .. } => body.leaf_count,
            Self::Branch8 { body, .. } => body.leaf_count,
            Self::Branch16 { body, .. } => body.leaf_count,
            Self::Branch32 { body, .. } => body.leaf_count,
            Self::Branch64 { body, .. } => body.leaf_count,
            Self::Branch128 { body, .. } => body.leaf_count,
            Self::Branch256 { body, .. } => body.leaf_count,
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

    fn with_start_depth(self, new_start_depth: usize, key: &[u8; KEY_LEN]) -> Head<KEY_LEN> {
        macro_rules! branchcase {
            ($start_depth:ident, $end_depth:ident, $fragment:ident, $body:ident, $variant:ident) => {{
                let actual_start_depth = max(
                    new_start_depth as isize,
                    $end_depth as isize - HEAD_FRAGMENT_LEN as isize,
                ) as usize;

                let mut new_fragment = [0; HEAD_FRAGMENT_LEN];
                for i in 0..new_fragment.len() {
                    let depth = actual_start_depth + i;
                    if($end_depth as usize <= depth) { break; }
                    new_fragment[i] = 
                        if depth < $start_depth as usize {
                            println!("key @ {}", depth);
                            key[depth]
                        } else {
                            $fragment[index_start($start_depth as usize, depth)]
                        }
                }
                Self::$variant {
                    start_depth: actual_start_depth as u8,
                    fragment: new_fragment,
                    end_depth: $end_depth,
                    body: $body,
                }
            }};
        }

        match self {
            Self::Empty { .. } => panic!("Called `expand` on `Empty."),
            Self::Leaf {head} => head.with_start_depth(new_start_depth, key),
            Self::Path14 {head} => head.with_start_depth(new_start_depth, key),
            Self::Path30 {head} => head.with_start_depth(new_start_depth, key),
            Self::Path46 {head} => head.with_start_depth(new_start_depth, key),
            Self::Path62 {head} => head.with_start_depth(new_start_depth, key),
            Self::Branch4 {
                start_depth, end_depth, fragment, body, ..
            } => branchcase!(start_depth, end_depth, fragment, body, Branch4),
            Self::Branch8 {
                start_depth, end_depth, fragment, body, ..
            } => branchcase!(start_depth, end_depth, fragment, body, Branch8),
            Self::Branch16 {
                start_depth, end_depth, fragment, body, ..
            } => branchcase!(start_depth, end_depth, fragment, body, Branch16),
            Self::Branch32 {
                start_depth, end_depth, fragment, body, ..
            } => branchcase!(start_depth, end_depth, fragment, body, Branch32),
            Self::Branch64 {
                start_depth, end_depth, fragment, body, ..
            } => branchcase!(start_depth, end_depth, fragment, body, Branch64),
            Self::Branch128 {
                start_depth, end_depth, fragment, body, ..
            } => branchcase!(start_depth, end_depth, fragment, body, Branch128),
            Self::Branch256 {
                start_depth, end_depth, fragment, body, ..
            } => branchcase!(start_depth, end_depth, fragment, body, Branch256),
        }
    }

    fn peek_branch(at_depth: usize, start_depth: usize, end_depth: usize, head_fragment: &[u8]) -> Option<u8> {
        if at_depth < start_depth || end_depth <= at_depth {
            return None;
        }
        return Some(head_fragment[index_start(start_depth, at_depth)]);
    }

    fn peek(&self, at_depth: usize) -> Option<u8> {
        match self {
            Self::Empty { .. } => panic!("Called `peek` on `Empty`."),
            Self::Leaf {head} => head.peek(at_depth),
            Self::Path14 {head} => head.peek(at_depth),
            Self::Path30 {head} => head.peek(at_depth),
            Self::Path46 {head} => head.peek(at_depth),
            Self::Path62 {head} => head.peek(at_depth),
            Self::Branch4 {
                start_depth,
                end_depth,
                fragment,
                ..
            } => Self::peek_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..]),
            Self::Branch8 {
                start_depth,
                end_depth,
                fragment,
                ..
            } => Self::peek_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..]),
            Self::Branch16 {
                start_depth,
                end_depth,
                fragment,
                ..
            } => Self::peek_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..]),
            Self::Branch32 {
                start_depth,
                end_depth,
                fragment,
                ..
            } => Self::peek_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..]),
            Self::Branch64 {
                start_depth,
                end_depth,
                fragment,
                ..
            } => Self::peek_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..]),
            Self::Branch128 {
                start_depth,
                end_depth,
                fragment,
                ..
            } => Self::peek_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..]),
            Self::Branch256 {
                start_depth,
                end_depth,
                fragment,
                ..
            } => Self::peek_branch(at_depth, *start_depth as usize, *end_depth as usize, &fragment[..]),
        }
    }

    fn propose_branch(at_depth: usize, start_depth: usize, end_depth: usize, head_fragment: &[u8], child_set: &ByteBitset, result_set: &mut ByteBitset) {
        if at_depth == end_depth {
            *result_set = *child_set;
            return;
        }

        result_set.unset_all();
        if let Some(byte_key) = Self::peek_branch(at_depth, start_depth, end_depth, head_fragment) {
            result_set.set(byte_key);
            return;
        }
    }

    fn propose(&self, at_depth: usize, result_set: &mut ByteBitset) {
        match self {
            Self::Empty { .. } => panic!("Called `propose` on `Empty`."),
            Self::Leaf {head} => head.propose(at_depth, result_set),
            Self::Path14 {head} => head.propose(at_depth, result_set),
            Self::Path30 {head} => head.propose(at_depth, result_set),
            Self::Path46 {head} => head.propose(at_depth, result_set),
            Self::Path62 {head} => head.propose(at_depth, result_set),
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
    fn put(self, at_start_depth: usize, key: &[u8; KEY_LEN]) -> Self {
        macro_rules! growinginsert {
            // TODO see if we can abstract the BranchN*2 logic away
            (Branch4, $start_depth:ident, $end_depth:ident, $fragment:ident, $body:ident, $inserted:ident) => {
                $inserted = $body.child_table.put($inserted);
                if $inserted.key().is_some() {
                    let mut new_body = $body.grow();
                    growinginsert!(
                        Branch8,
                        $start_depth,
                        $end_depth,
                        $fragment,
                        new_body,
                        $inserted
                    );
                } else {
                    return Self::Branch4 {
                        start_depth: $start_depth,
                        end_depth: $end_depth,
                        fragment: $fragment,
                        body: Arc::new($body),
                    };
                }
            };
            (Branch8, $start_depth:ident, $end_depth:ident, $fragment:ident, $body:ident, $inserted:ident) => {
                $inserted = $body.child_table.put($inserted);
                if $inserted.key().is_some() {
                    let mut new_body = $body.grow();
                    growinginsert!(
                        Branch16,
                        $start_depth,
                        $end_depth,
                        $fragment,
                        new_body,
                        $inserted
                    );
                } else {
                    return Self::Branch8 {
                        start_depth: $start_depth,
                        end_depth: $end_depth,
                        fragment: $fragment,
                        body: Arc::new($body),
                    };
                }
            };
            (Branch16, $start_depth:ident, $end_depth:ident, $fragment:ident, $body:ident, $inserted:ident) => {
                $inserted = $body.child_table.put($inserted);
                if $inserted.key().is_some() {
                    let mut new_body = $body.grow();
                    growinginsert!(
                        Branch32,
                        $start_depth,
                        $end_depth,
                        $fragment,
                        new_body,
                        $inserted
                    );
                } else {
                    return Self::Branch16 {
                        start_depth: $start_depth,
                        end_depth: $end_depth,
                        fragment: $fragment,
                        body: Arc::new($body),
                    };
                }
            };
            (Branch32, $start_depth:ident, $end_depth:ident, $fragment:ident, $body:ident, $inserted:ident) => {
                $inserted = $body.child_table.put($inserted);
                if $inserted.key().is_some() {
                    let mut new_body = $body.grow();
                    growinginsert!(
                        Branch64,
                        $start_depth,
                        $end_depth,
                        $fragment,
                        new_body,
                        $inserted
                    );
                } else {
                    return Self::Branch32 {
                        start_depth: $start_depth,
                        end_depth: $end_depth,
                        fragment: $fragment,
                        body: Arc::new($body),
                    };
                }
            };
            (Branch64, $start_depth:ident, $end_depth:ident, $fragment:ident, $body:ident, $inserted:ident) => {
                $inserted = $body.child_table.put($inserted);
                if $inserted.key().is_some() {
                    let mut new_body = $body.grow();
                    growinginsert!(
                        Branch128,
                        $start_depth,
                        $end_depth,
                        $fragment,
                        new_body,
                        $inserted
                    );
                } else {
                    return Self::Branch64 {
                        start_depth: $start_depth,
                        end_depth: $end_depth,
                        fragment: $fragment,
                        body: Arc::new($body),
                    };
                }
            };
            (Branch128, $start_depth:ident, $end_depth:ident, $fragment:ident, $body:ident, $inserted:ident) => {
                $inserted = $body.child_table.put($inserted);
                if $inserted.key().is_some() {
                    let mut new_body = $body.grow();
                    growinginsert!(
                        Branch256,
                        $start_depth,
                        $end_depth,
                        $fragment,
                        new_body,
                        $inserted
                    );
                } else {
                    return Self::Branch128 {
                        start_depth: $start_depth,
                        end_depth: $end_depth,
                        fragment: $fragment,
                        body: Arc::new($body),
                    };
                }
            };
            (Branch256, $start_depth:ident, $end_depth:ident, $fragment:ident, $body:ident, $inserted:ident) => {
                $body.child_table.put($inserted);
                return Self::Branch256 {
                    start_depth: $start_depth,
                    end_depth: $end_depth,
                    fragment: $fragment,
                    body: Arc::new($body),
                };
            };
        }

        macro_rules! branchcase {
            ($variant:ident, $start_depth: ident, $end_depth: ident, $fragment: ident, $body: ident) => {
                {
                let mut branch_depth = at_start_depth;
                while branch_depth < $end_depth as usize {
                    if key[branch_depth]
                        == $fragment[index_start($start_depth as usize, branch_depth)]
                    {
                        branch_depth += 1
                    } else {
                        break;
                    }
                }

                let mut new_body = Arc::try_unwrap($body).unwrap_or_else(|arc| (*arc).clone());

                if branch_depth == $end_depth as usize {
                    // The entire compressed infix above this node matched with the key.
                    let byte_key = key[branch_depth];
                    if new_body.child_set.is_set(byte_key) {
                        // The node already has a child branch with the same byte byte_key as the one in the key.
                        let old_child = new_body.child_table.take(byte_key).expect("table content should match child set content");
                        //let old_child_hash = old_child.hash(key);
                        //let old_child_segment_count = old_child.segmentCount(branch_depth);
                        //let new_child_hash = new_child.hash(key);
                        let old_child_leaf_count = old_child.count();
                        let new_child = old_child.put(branch_depth, key);

                        //let new_hash = self.body.node_hash.update(old_child_hash, new_child_hash);
                        //let new_segment_count = self.body.segment_count - old_child_segment_count + new_child.segmentCount(branch_depth);

                        //new_body.node_hash = new_hash;
                        //new_body.segment_count = new_segment_count;
                        new_body.leaf_count = (new_body.leaf_count - old_child_leaf_count as u64) + new_child.count() as u64;
                        new_body.child_table.put(new_child);

                        return Self::$variant {
                            start_depth: $start_depth,
                            end_depth: $end_depth,
                            fragment: $fragment,
                            body: Arc::new(new_body),
                        };
                    }
                    let mut inserted =
                    Self::newLeaf(branch_depth, key).wrap_path(branch_depth, key);

                    new_body.child_set.set(inserted.key().expect("leaf should have a byte key"));

                    growinginsert!($variant, $start_depth, $end_depth, $fragment, new_body, inserted);
                }

                let sibling_leaf_node =
                Self::newLeaf(branch_depth, key).wrap_path(branch_depth, key);

                let self_node = Self::$variant {
                    start_depth: $start_depth,
                    end_depth: $end_depth,
                    fragment: $fragment,
                    body: Arc::new(new_body),
                };

                let branch_head =
                Self::newBranch(at_start_depth, branch_depth, key, sibling_leaf_node, self_node.wrap_path(branch_depth, key));

                return branch_head.wrap_path(at_start_depth, key);
            }
        };
        }

        match self {
            Self::Empty { .. } => {
                Self::newLeaf(at_start_depth, key).wrap_path(at_start_depth, key)
            }
            Self::Leaf {head} => {head.put(at_start_depth, key)}
            Self::Path14 {head} => {head.put(at_start_depth, key)}
            Self::Path30 {head} => {head.put(at_start_depth, key)}
            Self::Path46 {head} => {head.put(at_start_depth, key)}
            Self::Path62 {head} => {head.put(at_start_depth, key)}
            Self::Branch4 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => branchcase!(Branch4, start_depth, end_depth, fragment, body),
            Self::Branch8 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => branchcase!(Branch8, start_depth, end_depth, fragment, body),
            Self::Branch16 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => branchcase!(Branch16, start_depth, end_depth, fragment, body),
            Self::Branch32 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => branchcase!(Branch32, start_depth, end_depth, fragment, body),
            Self::Branch64 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => branchcase!(Branch64, start_depth, end_depth, fragment, body),
            Self::Branch128 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => branchcase!(Branch128, start_depth, end_depth, fragment, body),
            Self::Branch256 {
                start_depth,
                end_depth,
                fragment,
                body,
            } => branchcase!(Branch256, start_depth, end_depth, fragment, body),
        }
    }
}

impl<const KEY_LEN: usize> Default for Head<KEY_LEN> {
    fn default() -> Self {
        Self::newEmpty()
    }
}

unsafe impl<const KEY_LEN: usize> ByteEntry for Head<KEY_LEN> {
    fn zeroed() -> Self {
        Self::newEmpty()
    }

    fn key(&self) -> Option<u8> {
        match self {
            Self::Empty { .. } => None,
            Self::Leaf { head } => Some(head.fragment[0]),
            Self::Path14 { head } => Some(head.fragment[0]),
            Self::Path30 { head } => Some(head.fragment[0]),
            Self::Path46 { head } => Some(head.fragment[0]),
            Self::Path62 { head } => Some(head.fragment[0]),
            Self::Branch4 { fragment, .. } => Some(fragment[0]),
            Self::Branch8 { fragment, .. } => Some(fragment[0]),
            Self::Branch16 { fragment, .. } => Some(fragment[0]),
            Self::Branch32 { fragment, .. } => Some(fragment[0]),
            Self::Branch64 { fragment, .. } => Some(fragment[0]),
            Self::Branch128 { fragment, .. } => Some(fragment[0]),
            Self::Branch256 { fragment, .. } => Some(fragment[0]),
        }
    }
}

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
            root: Head::<KEY_LEN>::newEmpty(),
        }
    }

    pub fn put(&mut self, key: [u8; KEY_LEN]) {
        let root = mem::take(&mut self.root);
        self.root = root.put(0, &key);
    }

    pub fn count(&self) -> u64 {
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

    #[test]
    fn head_size() {
        assert_eq!(mem::size_of::<Head<64>>(), 16);
    }

    #[test]
    fn empty_tree() {
        init();
        
        let tree = PACT::<64>::new();
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
        fn tree_put(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..256)) {
            let mut tree = PACT::<64>::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                tree.put(key);
            }
            //let entry_set = HashSet::from_iter(entries.iter().cloned());
        }
    }

}
