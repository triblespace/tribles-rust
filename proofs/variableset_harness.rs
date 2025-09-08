#![cfg(kani)]

use crate::query::VariableSet;

#[kani::proof]
#[kani::unwind(16)]
fn variableset_random() {
    let a: u8 = kani::any();
    let b: u8 = kani::any();
    let c: u8 = kani::any();
    // limit to small indexes
    kani::assume(a < 16 && b < 16 && c < 16);

    // first set
    let mut left = VariableSet::new_empty();
    left.set(a as usize);
    left.set(b as usize);
    // second set
    let mut right = VariableSet::new_empty();
    right.set(b as usize);
    right.set(c as usize);

    // expected bit patterns using u128
    let mut expected_left: u128 = 0;
    expected_left |= 1u128 << a;
    expected_left |= 1u128 << b;
    let mut expected_right: u128 = 0;
    expected_right |= 1u128 << b;
    expected_right |= 1u128 << c;

    // count after setting
    assert_eq!(left.count(), expected_left.count_ones() as usize);
    assert_eq!(right.count(), expected_right.count_ones() as usize);

    // unset b in left
    left.unset(b as usize);
    expected_left &= !(1u128 << b);
    assert_eq!(left.count(), expected_left.count_ones() as usize);

    // union
    let union = left.clone().union(right.clone());
    let expected_union = expected_left | expected_right;
    assert_eq!(union.count(), expected_union.count_ones() as usize);

    // intersection and intersects predicate
    let inter = left.intersect(right);
    let expected_inter = expected_left & expected_right;
    assert_eq!(inter.count(), expected_inter.count_ones() as usize);
    let intersects = !inter.is_empty();
    assert_eq!(intersects, expected_inter != 0);
}