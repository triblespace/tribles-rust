pub mod attributeconstraint;

use std::collections::{HashMap, HashSet};

use std::iter::FromIterator;
use std::marker::PhantomData;

use crate::query::Variable;
use crate::types::{Id, Idlike, Value, Valuelike};

use self::attributeconstraint::AttributeConstraint;

#[derive(Debug, Clone)]
pub struct Attribute<E, V>
where
    E: Idlike,
    V: Valuelike,
{
    pub ev: HashMap<Id, HashSet<Value>>,
    pub ve: HashMap<Value, HashSet<Id>>,
    pe: PhantomData<E>,
    pv: PhantomData<V>,
}

impl<E, V> Attribute<E, V>
where
    E: Idlike,
    V: Valuelike,
{
    pub fn new() -> Attribute<E, V> {
        Attribute {
            ev: HashMap::new(),
            ve: HashMap::new(),
            pe: PhantomData,
            pv: PhantomData,
        }
    }

    pub fn add(&mut self, e: &E, v: &V) {
        let id: Id = e.into_id();
        let value: Value = v.into_value();
        self.ev.entry(id).or_default().insert(value);
        self.ve.entry(value).or_default().insert(id);
    }

    pub fn has<'a>(&'a self, e: Variable<E>, v: Variable<V>) -> AttributeConstraint<'a, E, V> {
        AttributeConstraint::new(e, v, self)
    }
}

impl<E, V> FromIterator<(E, V)> for Attribute<E, V>
where
    E: Idlike,
    V: Valuelike,
{
    fn from_iter<I: IntoIterator<Item = (E, V)>>(iter: I) -> Self {
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
