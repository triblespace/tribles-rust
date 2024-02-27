pub mod attributeconstraint;

use std::collections::{HashMap, HashSet};

use std::iter::FromIterator;
use std::marker::PhantomData;

use crate::query::Variable;
use crate::{Id, Value, Valuelike};

use self::attributeconstraint::AttributeConstraint;

#[derive(Debug, Clone)]
pub struct Attribute<V: Valuelike> {
    pub ev: HashMap<Id, HashSet<Value>>,
    pub ve: HashMap<Value, HashSet<Id>>,
    pv: PhantomData<V>,
}

impl<V: Valuelike> Attribute<V> {
    pub fn new() -> Attribute<V> {
        Attribute {
            ev: HashMap::new(),
            ve: HashMap::new(),
            pv: PhantomData,
        }
    }

    pub fn add(&mut self, e: &Id, v: &V) {
        let value: Value = Valuelike::into_value(v);
        self.ev.entry(*e).or_default().insert(value);
        self.ve.entry(value).or_default().insert(*e);
    }

    pub fn has<'a>(&'a self, e: Variable<Id>, v: Variable<V>) -> AttributeConstraint<'a, V> {
        AttributeConstraint::new(e, v, self)
    }
}

impl<V: Valuelike> FromIterator<(Id, V)> for Attribute<V> {
    fn from_iter<I: IntoIterator<Item = (Id, V)>>(iter: I) -> Self {
        let mut attr = Attribute::new();

        for (e, v) in iter {
            attr.add(&e, &v);
        }
        attr
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn put(entries in prop::collection::vec((prop::num::u64::ANY, prop::num::u64::ANY), 1..1024)) {
            let set = Attribute::from_iter(entries.iter());
        }
    }
}
*/
