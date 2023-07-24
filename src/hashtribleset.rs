use std::collections::{HashMap, HashSet};

use crate::namespace::{Id, Value};
use crate::trible::Trible;
use std::iter::FromIterator;

#[derive(Debug, Clone)]
pub struct HashTribleSet {
    e: HashSet<Id>,
    a: HashSet<Id>,
    v: HashSet<Value>,
    ea: HashMap<Id, HashSet<Id>>,
    ev: HashMap<Id, HashSet<Value>>,
    ae: HashMap<Id, HashSet<Id>>,
    av: HashMap<Id, HashSet<Value>>,
    ve: HashMap<Value, HashSet<Id>>,
    va: HashMap<Value, HashSet<Id>>,
    eav: HashMap<(Id, Id), HashSet<Value>>,
    eva: HashMap<(Id, Value), HashSet<Id>>,
    ave: HashMap<(Id, Value), HashSet<Id>>,
    all: HashSet<Trible>,
}

impl HashTribleSet {
    pub fn new() -> HashTribleSet {
        HashTribleSet {
            e: HashSet::new(),
            a: HashSet::new(),
            v: HashSet::new(),
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

    pub fn add(&mut self, trible: &Trible) {
        let e = trible.e();
        let a = trible.a();
        let v = trible.v();
        self.e.insert(e);
        self.a.insert(a);
        self.v.insert(v);
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
            set.add(&t);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn put(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut set = HashTribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.add(&Trible{ data: key});
            }
        }
    }
}
