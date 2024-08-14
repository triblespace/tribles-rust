use std::convert::TryInto;

use crate::{ Value, Schema };

use hifitime::prelude::*;

pub struct NsTAIInterval;

impl Schema for NsTAIInterval {}

impl From<(Epoch, Epoch)> for Value<NsTAIInterval> {
    fn from(value: (Epoch, Epoch)) -> Self {
        let lower = value.0.to_tai_duration().total_nanoseconds();
        let upper = value.1.to_tai_duration().total_nanoseconds();

        let mut value = [0; 32];
        value[0..16].copy_from_slice(&lower.to_be_bytes());
        value[16..32].copy_from_slice(&upper.to_be_bytes());
        Value::new(value)
    }
}

impl From<Value<NsTAIInterval>> for (Epoch, Epoch) {
    fn from(value: Value<NsTAIInterval>) -> Self {
        let lower = i128::from_be_bytes(value.bytes[0..16].try_into().unwrap());
        let upper = i128::from_be_bytes(value.bytes[16..32].try_into().unwrap());
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
        let interval: Value<NsTAIInterval> = time_in.into();
        let time_out: (Epoch, Epoch) = interval.into();

        assert_eq!(time_in, time_out);
    }
}
