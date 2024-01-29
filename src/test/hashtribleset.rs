pub mod hashtriblesetconstraint;

use std::collections::{HashMap, HashSet};

use crate::query::TriblePattern;
use crate::trible::Trible;
use crate::types::{Id, Idlike, Value, Valuelike};
use std::iter::FromIterator;

use self::hashtriblesetconstraint::HashTribleSetConstraint;

#[derive(Debug, Clone)]
pub struct HashTribleSet {
    pub ea: HashMap<Id, HashSet<Id>>,
    pub ev: HashMap<Id, HashSet<Value>>,
    pub ae: HashMap<Id, HashSet<Id>>,
    pub av: HashMap<Id, HashSet<Value>>,
    pub ve: HashMap<Value, HashSet<Id>>,
    pub va: HashMap<Value, HashSet<Id>>,
    pub eav: HashMap<(Id, Id), HashSet<Value>>,
    pub eva: HashMap<(Id, Value), HashSet<Id>>,
    pub ave: HashMap<(Id, Value), HashSet<Id>>,
    pub all: HashSet<Trible>,
}

impl HashTribleSet {
    pub fn new() -> HashTribleSet {
        HashTribleSet {
            ea: HashMap::new(),
            ev: HashMap::new(),
            ae: HashMap::new(),
            av: HashMap::new(),
            ve: HashMap::new(),
            va: HashMap::new(),
            eav: HashMap::new(),
            eva: HashMap::new(),
            ave: HashMap::new(),
            all: HashSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        return self.all.len();
    }

    pub fn insert(&mut self, trible: &Trible) {
        let e = trible.e();
        let a = trible.a();
        let v = trible.v();
        self.ea.entry(e).or_default().insert(a);
        self.ev.entry(e).or_default().insert(v);
        self.ae.entry(a).or_default().insert(e);
        self.av.entry(a).or_default().insert(v);
        self.ve.entry(v).or_default().insert(e);
        self.va.entry(v).or_default().insert(a);
        self.eav.entry((e, a)).or_default().insert(v);
        self.eva.entry((e, v)).or_default().insert(a);
        self.ave.entry((a, v)).or_default().insert(e);
        self.all.insert(*trible);
    }
}

impl FromIterator<Trible> for HashTribleSet {
    fn from_iter<I: IntoIterator<Item = Trible>>(iter: I) -> Self {
        let mut set = HashTribleSet::new();

        for t in iter {
            set.insert(&t);
        }
        set
    }
}

impl TriblePattern for HashTribleSet {
    type PatternConstraint<'a, E, A, V>
     = HashTribleSetConstraint<'a, E, A, V>
     where
     E: Idlike,
     A: Idlike,
     V: Valuelike;

    fn pattern<'a, E, A, V>(
        &'a self,
        e: crate::query::Variable<E>,
        a: crate::query::Variable<A>,
        v: crate::query::Variable<V>,
    ) -> Self::PatternConstraint<'a, E, A, V>
    where
        E: Idlike,
        A: Idlike,
        V: Valuelike,
    {
        HashTribleSetConstraint::new(e, a, v, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn insert(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut set = HashTribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.insert(&Trible{ data: key});
            }
        }
    }
}
