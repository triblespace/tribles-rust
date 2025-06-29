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
