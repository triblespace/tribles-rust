#![cfg(kani)]

use tribles::value::{schemas::ShortString, Value};
use tribles::value::{TryFromValue, ValueSchema};

#[kani::proof]
fn short_string_roundtrip() {
    let value: Value<ShortString> = ShortString::value_from("hello");
    let result: Result<&str, _> = value.try_from_value();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}
