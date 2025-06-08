#![cfg(kani)]

use crate::id::{rngid, Id};

#[kani::proof]
fn acquire_release_roundtrip() {
    // Mint an exclusive id and release it so it becomes owned by this thread
    let ex = rngid();
    let id = ex.release();

    // Acquire the id again and release it once more
    let ex2 = id.aquire().expect("id should be owned after release");
    let id2 = ex2.release();

    // The id value must be preserved
    assert_eq!(id, id2);
}

#[kani::proof]
fn id_bytes_roundtrip() {
    let raw: [u8; 16] = [0xAA; 16];
    let id = unsafe { Id::force(raw) };
    let bytes: [u8; 16] = id.into();
    assert_eq!(raw, bytes);
}
