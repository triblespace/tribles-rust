pub mod attrconstraint;

use std::collections::{HashMap, HashSet};

use std::iter::FromIterator;

use self::attrconstraint::AttrConstraint;

use super::TribleSet;

#[derive(Debug, Clone)]
pub struct Attribute<Id, Value> {
    pub ve: HashMap<Value, HashSet<Id>>,
    pub ev: HashMap<Id, HashSet<Value>>,
}

impl<Id, Value> Attribute<Id, Value> {
    pub fn new() -> Attribute {
        Attribute {
            ev: HashMap::new(),
            ve: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        return self.all.len();
    }

    pub fn add(&mut self, ev: &(Id, Value)) {
        let (e, v) = ev;
        self.ev.entry(e).or_default().insert(v);
        self.ve.entry(v).or_default().insert(e);
    }

    pub fn has<'a>(&'a mut self, e: &Variable<Id>, v: Variable<Value>) -> AttrConstraint<'a, Id, Value> {
        HashTribleSetConstraint::new(e, v, self)
    }
}

impl<Id, Value> FromIterator<(Id, Value)> for Attribute<Id, Value> {
    fn from_iter<I: IntoIterator<Item = (Id, Value)>>(iter: I) -> Self {
        let mut attr = Attribute::new();

        for t in iter {
            attr.add(&t);
        }
        attr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn put(entries in prop::collection::vec((prop::num::u64, prop::num::u64), 1..1024)) {
            let set = Attribute::from_iter(entries.iter());
        }
    }
}
