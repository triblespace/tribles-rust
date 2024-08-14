pub mod hashtriblesetconstraint;

use std::collections::{HashMap, HashSet};

use crate::query::TriblePattern;
use crate::trible::Trible;
use crate::{Id, RawId, RawValue, Schema};
use std::iter::FromIterator;

use self::hashtriblesetconstraint::HashTribleSetConstraint;

#[derive(Debug, Clone)]
pub struct HashTribleSet {
    pub ea: HashMap<RawId, HashSet<RawId>>,
    pub ev: HashMap<RawId, HashSet<RawValue>>,
    pub ae: HashMap<RawId, HashSet<RawId>>,
    pub av: HashMap<RawId, HashSet<RawValue>>,
    pub ve: HashMap<RawValue, HashSet<RawId>>,
    pub va: HashMap<RawValue, HashSet<RawId>>,
    pub eav: HashMap<(RawId, RawId), HashSet<RawValue>>,
    pub eva: HashMap<(RawId, RawValue), HashSet<RawId>>,
    pub ave: HashMap<(RawId, RawValue), HashSet<RawId>>,
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
    type PatternConstraint<'a>
     = HashTribleSetConstraint<'a>;

    fn pattern<'a, V: Schema>(
        &'a self,
        e: crate::query::Variable<Id>,
        a: crate::query::Variable<Id>,
        v: crate::query::Variable<V>,
    ) -> Self::PatternConstraint<'a>
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
