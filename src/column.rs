pub mod columnconstraint;

use std::collections::{HashMap, HashSet};

use std::iter::FromIterator;

use crate::query::Variable;
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

impl<V: ValueSchema> Column<V> {
    pub fn new() -> Self {
        Self {
            ev: HashMap::new(),
            ve: HashMap::new(),
        }
    }

    pub fn insert(&mut self, e: RawId, v: Value<V>) {
        self.ev.entry(e).or_default().insert(v);
        self.ve.entry(v).or_default().insert(e);
    }

    pub fn remove(&mut self, e: RawId, v: Value<V>) {
        self.ev.entry(e).or_default().remove(&v);
        self.ve.entry(v).or_default().remove(&e);
    }

    pub fn has<'a>(&'a self, e: Variable<GenId>, v: Variable<V>) -> ColumnConstraint<'a, V> {
        ColumnConstraint::new(e, v, self)
    }
}

impl<'a, V: ValueSchema> FromIterator<&'a (RawId, Value<V>)> for Column<V> {
    fn from_iter<I: IntoIterator<Item = &'a (RawId, Value<V>)>>(iter: I) -> Self {
        let mut column = Self::new();

        for &(k, v) in iter {
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
