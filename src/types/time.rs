use crate::Valuelike;

use uom::si::i128::Time;
use uom::si::time::nanosecond;

/*
impl<Tz> Valuelike for DateTime<Tz>
where Tz: TimeZone {
    fn from_value(value: crate::Value) -> Result<Self, crate::ValueParseError> {
        todo!()
    }

    fn into_value(v: &Self) -> crate::Value {
        todo!()
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nanosecond_interval() {
        let time = Time::new::<nanosecond>(1);
        let v = time.value;
        println!("{} nanos", v);
    }
}
