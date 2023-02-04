use crate::pact::PACT;
use crate::trible::Trible;

#[derive(Debug, Clone)]
pub struct TribleSet {
    eav: PACT<64>,
    aev: PACT<64>,
    ave: PACT<64>,
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

        TribleSet {
            eav,
            aev,
            ave,
        }
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

    pub fn put(&mut self, trible: &Trible) {
        self.eav.put(trible.order_eav());
        self.aev.put(trible.order_aev());
        self.ave.put(trible.order_ave());
    }
}
