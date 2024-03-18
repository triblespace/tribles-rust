use std::convert::TryInto;

use crate::Valuelike;

use hifitime::prelude::*;

pub struct NsTAIInterval(pub i128, pub i128);

impl Valuelike for NsTAIInterval {
    fn from_value(bytes: crate::Value) -> Result<Self, crate::ValueParseError> {
        let lower = i128::from_be_bytes(bytes[0..16].try_into().unwrap());
        let upper = i128::from_be_bytes(bytes[16..32].try_into().unwrap());
        Ok(NsTAIInterval(lower, upper))
    }

    fn into_value(interval: &Self) -> crate::Value {
        let mut value = [0; 32];
        value[0..16].copy_from_slice(&interval.0.to_be_bytes());
        value[16..32].copy_from_slice(&interval.1.to_be_bytes());
        value
    }
}

impl From<(Epoch, Epoch)> for NsTAIInterval {
    fn from(value: (Epoch, Epoch)) -> Self {
        let lower = value.0.to_tai_duration().total_nanoseconds();
        let upper = value.1.to_tai_duration().total_nanoseconds();

        NsTAIInterval(lower, upper)
    }
}

impl From<NsTAIInterval> for (Epoch, Epoch) {
    fn from(value: NsTAIInterval) -> Self {
        let lower = Epoch::from_tai_duration(Duration::from_total_nanoseconds(value.0));
        let upper = Epoch::from_tai_duration(Duration::from_total_nanoseconds(value.1));

        (lower, upper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tai_nanosecond_interval() {
        let epoch: NsTAIInterval = NsTAIInterval(0, 0);
        let value: [u8; 32] = NsTAIInterval::into_value(&epoch);
        let _ = NsTAIInterval::from_value(value);
    }

    #[test]
    fn hifitime_conversion() {
        let epoch: NsTAIInterval = NsTAIInterval(0, 0);
        let time: (Epoch, Epoch) = epoch.into();
        let _: NsTAIInterval = time.into();
    }
}
