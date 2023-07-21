mod tribleconstraint;

use tribleconstraint::*;

use crate::pact::PACT;
use crate::trible::{TribleSegmentation, AEVOrder, AVEOrder, EAVOrder, EVAOrder, Trible, VAEOrder, VEAOrder};
use std::iter::FromIterator;
use triomphe::Arc;

#[derive(Debug, Clone)]
pub struct TribleSet {
    eav: PACT<64, EAVOrder, TribleSegmentation>,
    eva: PACT<64, EVAOrder, TribleSegmentation>,
    aev: PACT<64, AEVOrder, TribleSegmentation>,
    ave: PACT<64, AVEOrder, TribleSegmentation>,
    vea: PACT<64, VEAOrder, TribleSegmentation>,
    vae: PACT<64, VAEOrder, TribleSegmentation>,
}

impl TribleSet {
    pub fn union<I>(sets: I) -> TribleSet
    where
        I: IntoIterator<Item = TribleSet>,
        I::IntoIter: Clone,
    {
        let iter = sets.into_iter();
        let eav = PACT::union(iter.clone().map(|set| set.eav));
        let eva = PACT::union(iter.clone().map(|set| set.eva));
        let aev = PACT::union(iter.clone().map(|set| set.aev));
        let ave = PACT::union(iter.clone().map(|set| set.ave));
        let vea = PACT::union(iter.clone().map(|set| set.vea));
        let vae = PACT::union(iter.clone().map(|set| set.vae));

        TribleSet {
            eav,
            eva,
            aev,
            ave,
            vea,
            vae,
        }
    }

    pub fn new() -> TribleSet {
        TribleSet {
            eav: PACT::new(),
            eva: PACT::new(),
            aev: PACT::new(),
            ave: PACT::new(),
            vea: PACT::new(),
            vae: PACT::new(),
        }
    }

    pub fn len(&self) -> u32 {
        return self.eav.len();
    }

    pub fn add(&mut self, trible: &Trible) {
        let key = Arc::new(trible.data);
        self.eav.put(&key);
        self.eva.put(&key);
        self.aev.put(&key);
        self.ave.put(&key);
        self.vea.put(&key);
        self.vae.put(&key);
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
