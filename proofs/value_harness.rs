#![cfg(kani)]

use crate::value::schemas::shortstring::ShortString;
use crate::value::TryFromValue;
use crate::value::Value;
use crate::value::ValueSchema;

#[kani::proof]
#[kani::unwind(33)]
fn short_string_roundtrip() {
    let raw: [u8; 32] = kani::any();
    let value: Value<ShortString> = Value::new(raw);
    kani::assume(value.is_valid());

    let s: &str = value.try_from_value().unwrap();
    let roundtrip = ShortString::value_from(s);
    assert_eq!(value, roundtrip);
}
