#![cfg(kani)]

// A sample proof considered "slow" and gated behind the slowproofs feature.
#[kani::proof]
fn example_slow_proof() {
    // This test simply asserts true but could represent a heavy check
    kani::assume(true);
    assert!(true);
}
