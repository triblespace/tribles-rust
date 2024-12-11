use crate::id::Id;
use crate::id_hex;
use crate::value::{FromValue, ToValue, Value, ValueSchema};

use std::convert::TryInto;

use hifitime::prelude::*;

pub struct NsTAIInterval;

impl ValueSchema for NsTAIInterval {
    const VALUE_SCHEMA_ID: Id = id_hex!("675A2E885B12FCBC0EEC01E6AEDD8AA8");
}

impl ToValue<NsTAIInterval> for (Epoch, Epoch) {
    fn to_value(self) -> Value<NsTAIInterval> {
        let lower = self.0.to_tai_duration().total_nanoseconds();
        let upper = self.1.to_tai_duration().total_nanoseconds();

        let mut value = [0; 32];
        value[0..16].copy_from_slice(&lower.to_be_bytes());
        value[16..32].copy_from_slice(&upper.to_be_bytes());
        Value::new(value)
    }
}

impl FromValue<'_, NsTAIInterval> for (Epoch, Epoch) {
    fn from_value(v: &Value<NsTAIInterval>) -> Self {
        let lower = i128::from_be_bytes(v.bytes[0..16].try_into().unwrap());
        let upper = i128::from_be_bytes(v.bytes[16..32].try_into().unwrap());
        let lower = Epoch::from_tai_duration(Duration::from_total_nanoseconds(lower));
        let upper = Epoch::from_tai_duration(Duration::from_total_nanoseconds(upper));

        (lower, upper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hifitime_conversion() {
        let now = Epoch::now().unwrap();
        let time_in: (Epoch, Epoch) = (now, now);
        let interval: Value<NsTAIInterval> = time_in.to_value();
        let time_out: (Epoch, Epoch) = interval.from_value();

        assert_eq!(time_in, time_out);
    }
}
