pub mod columnconstraint;

use std::collections::{HashMap, HashSet};

use std::iter::FromIterator;

use crate::query::Variable;
use crate::value::ToValue;
use crate::{
    id::RawId,
    value::schemas::genid::GenId,
    value::{Value, ValueSchema},
};

use self::columnconstraint::ColumnConstraint;

#[derive(Debug, Clone)]
pub struct Column<S: ValueSchema> {
    pub ev: HashMap<RawId, HashSet<Value<S>>>,
    pub ve: HashMap<Value<S>, HashSet<RawId>>,
}

impl<S: ValueSchema> Column<S> {
    pub fn new() -> Self {
        Self {
            ev: HashMap::new(),
            ve: HashMap::new(),
        }
    }

    pub fn insert<T>(&mut self, e: &RawId, v: T)
    where T: ToValue<S> {
        let e = *e;
        let v = v.to_value();
        self.ev.entry(e).or_default().insert(v);
        self.ve.entry(v).or_default().insert(e);
    }

    pub fn remove<T>(&mut self, e: &RawId, v: T)
    where T: ToValue<S> {
        let e = *e;
        let v = v.to_value();
        self.ev.entry(e).or_default().remove(&v);
        self.ve.entry(v).or_default().remove(&e);
    }

    pub fn has<'a>(&'a self, e: Variable<GenId>, v: Variable<S>) -> ColumnConstraint<'a, S> {
        ColumnConstraint::new(e, v, self)
    }
}

impl<'a, S, T> FromIterator<&'a (RawId, T)> for Column<S>
where S: ValueSchema,
      for<'b> &'b T: ToValue<S> {
    fn from_iter<I: IntoIterator<Item = &'a (RawId, T)>>(iter: I) -> Self {
        let mut column = Self::new();

        for (k, v) in iter {
            let v = v.to_value();
            column.insert(k, v);
        }
        column
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{schemas::genid::RandomGenId, ToValue};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn insert(entries in prop::collection::vec((RandomGenId(), RandomGenId().prop_map(|id| id.to_value())), 1..1024)) {
            Column::from_iter(entries.iter());
        }
    }
}
