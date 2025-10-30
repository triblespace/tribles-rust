use crate::id::Id;
use crate::id_hex;
use crate::value::FromValue;
use crate::value::RawValue;
use crate::value::ToValue;
use crate::value::TryFromValue;
use crate::value::TryToValue;
use crate::value::Value;
use crate::value::ValueSchema;
use std::convert::Infallible;
use std::ops::{Range, RangeInclusive};

/// A value schema for representing a pair of `u128` values.
///
/// [`RangeU128`] encodes the pair as a half-open interval while
/// [`RangeInclusiveU128`] represents an inclusive range. Both schemas encode the
/// endpoints by packing the line into the high 64 bits and the column into the
/// low 64 bits of the `u128`.
#[derive(Debug, Clone, Copy)]
pub struct RangeU128;

#[derive(Debug, Clone, Copy)]
pub struct RangeInclusiveU128;

impl ValueSchema for RangeU128 {
    const VALUE_SCHEMA_ID: Id = id_hex!("A4E25E3B92364FA5AB519C6A77D7CB3A");
    type ValidationError = Infallible;
}

impl ValueSchema for RangeInclusiveU128 {
    const VALUE_SCHEMA_ID: Id = id_hex!("1D0D82CA84424CD0A2F98DB37039E152");
    type ValidationError = Infallible;
}

fn encode_pair(range: (u128, u128)) -> RawValue {
    let mut raw = [0u8; 32];
    raw[..16].copy_from_slice(&range.0.to_be_bytes());
    raw[16..].copy_from_slice(&range.1.to_be_bytes());
    raw
}

fn decode_pair(raw: &RawValue) -> (u128, u128) {
    let mut first = [0u8; 16];
    let mut second = [0u8; 16];
    first.copy_from_slice(&raw[..16]);
    second.copy_from_slice(&raw[16..]);
    (u128::from_be_bytes(first), u128::from_be_bytes(second))
}

fn encode_range_value<S: ValueSchema>(range: (u128, u128)) -> Value<S> {
    Value::new(encode_pair(range))
}

fn decode_range_value<S: ValueSchema>(value: &Value<S>) -> (u128, u128) {
    decode_pair(&value.raw)
}

impl ToValue<RangeU128> for (u128, u128) {
    fn to_value(self) -> Value<RangeU128> {
        encode_range_value(self)
    }
}

impl FromValue<'_, RangeU128> for (u128, u128) {
    fn from_value(v: &Value<RangeU128>) -> Self {
        decode_range_value(v)
    }
}

impl ToValue<RangeInclusiveU128> for (u128, u128) {
    fn to_value(self) -> Value<RangeInclusiveU128> {
        encode_range_value(self)
    }
}

impl FromValue<'_, RangeInclusiveU128> for (u128, u128) {
    fn from_value(v: &Value<RangeInclusiveU128>) -> Self {
        decode_range_value(v)
    }
}

impl TryToValue<RangeU128> for Range<u128> {
    type Error = Infallible;

    fn try_to_value(self) -> Result<Value<RangeU128>, Self::Error> {
        Ok(encode_range_value((self.start, self.end)))
    }
}

impl TryFromValue<'_, RangeU128> for Range<u128> {
    type Error = Infallible;

    fn try_from_value(v: &Value<RangeU128>) -> Result<Self, Self::Error> {
        let (start, end) = decode_range_value(v);
        Ok(start..end)
    }
}

impl TryToValue<RangeInclusiveU128> for RangeInclusive<u128> {
    type Error = Infallible;

    fn try_to_value(self) -> Result<Value<RangeInclusiveU128>, Self::Error> {
        let (start, end) = self.into_inner();
        Ok(encode_range_value((start, end)))
    }
}

impl TryFromValue<'_, RangeInclusiveU128> for RangeInclusive<u128> {
    type Error = Infallible;

    fn try_from_value(v: &Value<RangeInclusiveU128>) -> Result<Self, Self::Error> {
        let (start, end) = decode_range_value(v);
        Ok(start..=end)
    }
}
