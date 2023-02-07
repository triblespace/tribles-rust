use crate::pact::{ PACT, KeyProperties};
use crate::trible::Trible;
use std::iter::FromIterator;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TribleSet {
    eav: PACT<64, OrderEAV>,
    aev: PACT<64, OrderEAV>,
    ave: PACT<64, OrderEAV>,
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

#[derive(Copy, Clone, Debug)]
pub struct OrderEAV {}

impl<const KEY_LEN: usize> KeyProperties<KEY_LEN> for OrderEAV {
    fn reorder(depth: usize) -> usize {
        depth
    }
}