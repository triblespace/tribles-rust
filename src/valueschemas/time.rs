use std::convert::TryInto;

use crate::{Value, ValueSchema};

use hifitime::prelude::*;

use super::{PackValue, UnpackValue};

use hex_literal::hex;

pub struct NsTAIInterval;

impl ValueSchema for NsTAIInterval {const ID: crate::RawId = hex!("675A2E885B12FCBC0EEC01E6AEDD8AA8");}

impl PackValue<NsTAIInterval> for (Epoch, Epoch) {
    fn pack(&self) -> Value<NsTAIInterval> {
        let lower = self.0.to_tai_duration().total_nanoseconds();
        let upper = self.1.to_tai_duration().total_nanoseconds();

        let mut value = [0; 32];
        value[0..16].copy_from_slice(&lower.to_be_bytes());
        value[16..32].copy_from_slice(&upper.to_be_bytes());
        Value::new(value)
    }
}

impl UnpackValue<'_, NsTAIInterval> for (Epoch, Epoch) {
    fn unpack(v: &Value<NsTAIInterval>) -> Self {
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
        let interval: Value<NsTAIInterval> = time_in.pack();
        let time_out: (Epoch, Epoch) = interval.unpack();

        assert_eq!(time_in, time_out);
    }
}
