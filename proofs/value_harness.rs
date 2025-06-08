#![cfg(kani)]

use crate::value::{schemas::shortstring::ShortString, Value};
use crate::value::{TryFromValue, ValueSchema};

#[kani::proof]
fn short_string_roundtrip() {
    let value: Value<ShortString> = ShortString::value_from("hello");
    let result: Result<&str, _> = value.try_from_value();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}
