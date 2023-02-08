use crate::pact::{ PACT, KeyProperties};
use crate::trible::{Trible, EAVOrder, AEVOrder, AVEOrder};
use std::iter::FromIterator;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TribleSet {
    eav: PACT<64, EAVOrder>,
    aev: PACT<64, AEVOrder>,
    ave: PACT<64, AVEOrder>,
}

impl TribleSet {
    pub fn union<I>(sets: I) -> TribleSet
    where
        I: IntoIterator<Item = TribleSet>,
        I::IntoIter: Clone,
    {
        let iter = sets.into_iter();
        let eav = PACT::union(iter.clone().map(|set| set.eav));
        let aev = PACT::union(iter.clone().map(|set| set.aev));
        let ave = PACT::union(iter.clone().map(|set| set.ave));

        TribleSet { eav, aev, ave }
    }

    pub fn new() -> TribleSet {
        TribleSet {
            eav: PACT::new(),
            aev: PACT::new(),
            ave: PACT::new(),
        }
    }

    pub fn len(&self) -> u64 {
        return self.eav.len();
    }

    pub fn add(&mut self, trible: &Trible) {
        let key = Arc::new(trible.data);
        self.eav.put(&key);
        self.aev.put(&key);
        self.ave.put(&key);
    }
}

impl FromIterator<Trible> for TribleSet {
    fn from_iter<I: IntoIterator<Item = Trible>>(iter: I) -> Self {
        let mut set = TribleSet::new();

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
    use std::collections::HashSet;
    use std::iter::FromIterator;

    proptest! {
        #[test]
        fn tree_put(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut set = TribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.add(&Trible{ data: key});
            }
        }
    }
}
