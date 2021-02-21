use std::alloc::{alloc, dealloc, Layout};
use std::mem::MaybeUninit;
use std::ptr;

/*
#[cfg(target_pointer_width = "64")]
struct Node {
    hash: u128,    // 16 byte
    ref_count: Cell<u32>,    // 2 byte => every 2^32 references we need to copy 1 node.
    has_child: u16,    // 2 byte
    prefix_length: u8, // 1 byte
    reserved: u8, // 1 byte
    key_count: [u8; 6]  // 6 byte
    children: [u8; 84] // 84 byte
    prefix: [u8; 16]  // 16 byte
}
*/

/*
Note, [x x y] can be seen as a compound index key. E=A ->V
Searchign for both E and A at once.

> E -> A -> V1 -> V2
    -> V1 -> V2 -> A
> A -> E -> V1 -> V2
    -> V1 -> V2 -> E
> V1 -> V2 -> E -> A
           -> A -> E
> E=A
> A=V
> E=V
> E=A=V

*/
#[cfg(target_pointer_width = "64")]
#[repr(align(128))]
struct Node {
    ref_count: Cell<u32>, // 4 byte
    key_count: u64,   // 8 byte
    hash: u128,    // 16 byte TODO: use instance keyed xxHash
    mask_bits: u128, // 16 bytes
    common_bits: u128, // 16 bytes
    children: [u64; 16], // 128 byte top 16 bit are partial key;
}

#[cfg(target_pointer_width = "64")]
#[repr(align(128))]
struct Node {
    hash: u128,    // 16 byte TODO: use instance keyed xxHash
    ref_count: Cell<u32>, // 4 byte
    key_count: u64,   // 8 byte
    branch_has_child: u16,    // 2 byte
    standin_key: [MaybeUninit<u8>; 16], // 16 byes
    children: [u8; 82], // 82 byte
}

#[cfg(target_pointer_width = "32")]
struct Node {
    hash: u128,    // 16 byte TODO: use instance keyed xxHash
    ref_count: Cell<u16>, // 2 byte
    key_count: u32,   // 4 byte
    branch_position: u8,    // 1 byte, could be a nibble
    branch_bits: u8,      // 1 byte
    branch_has_child: u16,    // 2 byte
    standin_key: [MaybeUninit<u8>; 16], // 16 byes
    children: [u8; 54], // 54 byte
}

impl Node {
    unsafe fn new() -> *mut Node {
        let layout = Layout::new::<Node>();
        let node = alloc(layout) as *mut Node;
        ptr::write(ptr::addr_of_mut!(node.hash), [0; 3]);
        ptr::write(ptr::addr_of_mut!(node.ref_count), 0);
        ptr::write(ptr::addr_of_mut!(node.has_child), 0);
        ptr::write(ptr::addr_of_mut!(node.is_leaf), 0);
        ptr::write(ptr::addr_of_mut!(node.prefix_length), 0);
        ptr::write(ptr::addr_of_mut!(node.reserved), 0);
        node
    }
    
}
