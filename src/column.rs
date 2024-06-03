pub mod columnconstraint;

use std::collections::{HashMap, HashSet};

use std::iter::FromIterator;
use std::marker::PhantomData;

use crate::query::Variable;
use crate::{Id, Value, Valuelike};

use self::columnconstraint::ColumnConstraint;

#[derive(Debug, Clone)]
pub struct Column<V: Valuelike> {
    pub ev: HashMap<Id, HashSet<Value>>,
    pub ve: HashMap<Value, HashSet<Id>>,
    pv: PhantomData<V>,
}

impl<V: Valuelike> Column<V> {
    pub fn new() -> Self {
        Self {
            ev: HashMap::new(),
            ve: HashMap::new(),
            pv: PhantomData,
        }
    }

    pub fn insert(&mut self, e: &Id, v: &V) {
        let value: Value = Valuelike::into_value(v);
        self.ev.entry(*e).or_default().insert(value);
        self.ve.entry(value).or_default().insert(*e);
    }

    pub fn remove(&mut self, e: &Id, v: &V) {
        let value: Value = Valuelike::into_value(v);
        self.ev.entry(*e).or_default().remove(&value);
        self.ve.entry(value).or_default().remove(e);
    }

    pub fn has<'a>(&'a self, e: Variable<Id>, v: Variable<V>) -> ColumnConstraint<'a, V> {
        ColumnConstraint::new(e, v, self)
    }
}

impl<'a, V> FromIterator<&'a (Id, V)> for Column<V>
where
    V: Valuelike,
{
    fn from_iter<I: IntoIterator<Item = &'a (Id, V)>>(iter: I) -> Self {
        let mut column = Self::new();

        for (e, v) in iter {
            column.insert(e, v);
        }
        column
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn put(entries in prop::collection::vec((crate::id::RandId(), crate::id::RandId()), 1..1024)) {
            Column::from_iter(entries.iter());
        }
    }
}
