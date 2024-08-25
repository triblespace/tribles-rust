pub mod columnconstraint;

use std::collections::{HashMap, HashSet};

use std::iter::FromIterator;
use std::marker::PhantomData;

use crate::query::Variable;
use crate::{valueschemas::GenId, RawId, RawValue, ValueSchema, Value};

use self::columnconstraint::ColumnConstraint;

#[derive(Debug, Clone)]
pub struct Column<V> {
    pub ev: HashMap<RawId, HashSet<RawValue>>,
    pub ve: HashMap<RawValue, HashSet<RawId>>,
    pv: PhantomData<V>,
}

impl<V: ValueSchema> Column<V> {
    pub fn new() -> Self {
        Self {
            ev: HashMap::new(),
            ve: HashMap::new(),
            pv: PhantomData,
        }
    }

    pub fn insert(&mut self, e: RawId, v: Value<V>) {
        self.ev.entry(e).or_default().insert(v.bytes);
        self.ve.entry(v.bytes).or_default().insert(e);
    }

    pub fn remove(&mut self, e: RawId, v: Value<V>) {
        self.ev.entry(e).or_default().remove(&v.bytes);
        self.ve.entry(v.bytes).or_default().remove(&e);
    }

    pub fn has<'a>(&'a self, e: Variable<GenId>, v: Variable<V>) -> ColumnConstraint<'a> {
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
    use crate::valueschemas::genid::RandomGenId;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn insert(entries in prop::collection::vec((RandomGenId(), RandomGenId().prop_map(|id| id.into())), 1..1024)) {
            Column::from_iter(entries.iter());
        }
    }
}
