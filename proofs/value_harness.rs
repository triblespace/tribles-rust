#![cfg(kani)]

use crate::value::{schemas::shortstring::ShortString, Value};
use crate::value::{TryFromValue, ValueSchema};
use kani::BoundedArbitrary;

#[kani::proof]
#[kani::unwind(32)]
fn short_string_roundtrip() {
    let s = String::bounded_any::<32>();
    let value: Value<ShortString> = ShortString::value_from(&s);
    let result: Result<&str, _> = value.try_from_value();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), s.as_str());
}

/// Test generated for harness `proofs::value_harness::short_string_roundtrip`
///
/// Check for `assertion`: "This is a placeholder message; Kani doesn't support message formatted at runtime"

#[test]
fn kani_concrete_playback_short_string_roundtrip_10333782728379354304() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 0
        vec![0],
        // 128
        vec![128],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 192
        vec![192],
        // 127
        vec![127],
        // 193
        vec![193],
        // 192
        vec![192],
        // 127
        vec![127],
        // 192
        vec![192],
        // 192
        vec![192],
        // 127
        vec![127],
        // 192
        vec![192],
        // 192
        vec![192],
        // 127
        vec![127],
        // 192
        vec![192],
        // 192
        vec![192],
        // 127
        vec![127],
        // 192
        vec![192],
        // 192
        vec![192],
        // 127
        vec![127],
        // 255
        vec![255],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
    ];
    kani::concrete_playback_run(concrete_vals, short_string_roundtrip);
}
