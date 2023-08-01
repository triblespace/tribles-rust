mod pacttribleconstraint;

use pacttribleconstraint::*;

use crate::namespace::{Id, Value};
use crate::pact::PACT;
use crate::trible::{
    AEVOrder, AVEOrder, EAVOrder, EVAOrder, Trible, TribleSegmentation, VAEOrder, VEAOrder,
};
use std::iter::FromIterator;
use triomphe::Arc;

use super::TribleSet;

#[derive(Debug, Clone)]
pub struct PACTTribleSet {
    eav: PACT<64, EAVOrder, TribleSegmentation>,
    eva: PACT<64, EVAOrder, TribleSegmentation>,
    aev: PACT<64, AEVOrder, TribleSegmentation>,
    ave: PACT<64, AVEOrder, TribleSegmentation>,
    vea: PACT<64, VEAOrder, TribleSegmentation>,
    vae: PACT<64, VAEOrder, TribleSegmentation>,
}

impl PACTTribleSet {
    pub fn union<I>(sets: I) -> PACTTribleSet
    where
        I: IntoIterator<Item = PACTTribleSet>,
        I::IntoIter: Clone,
    {
        let iter = sets.into_iter();
        let eav = PACT::union(iter.clone().map(|set| set.eav));
        let eva = PACT::union(iter.clone().map(|set| set.eva));
        let aev = PACT::union(iter.clone().map(|set| set.aev));
        let ave = PACT::union(iter.clone().map(|set| set.ave));
        let vea = PACT::union(iter.clone().map(|set| set.vea));
        let vae = PACT::union(iter.clone().map(|set| set.vae));

        PACTTribleSet {
            eav,
            eva,
            aev,
            ave,
            vea,
            vae,
        }
    }

    pub fn new() -> PACTTribleSet {
        PACTTribleSet {
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

impl FromIterator<Trible> for PACTTribleSet {
    fn from_iter<I: IntoIterator<Item = Trible>>(iter: I) -> Self {
        let mut set = PACTTribleSet::new();

        for t in iter {
            set.add(&t);
        }

        set
    }
}

impl TribleSet for PACTTribleSet {
    type PatternConstraint<'a, E, A, V>
     = PACTTribleSetConstraint<'a, E, A, V>
     where
     E: From<Id>,
     A: From<Id>,
     V: From<Value>,
     for<'b> &'b E: Into<Id>,
     for<'b> &'b A: Into<Id>,
     for<'b> &'b V: Into<Value>;


    fn pattern<'a, E, A, V>(&'a self, e: crate::query::Variable<E>, a: crate::query::Variable<A>, v: crate::query::Variable<V>) -> Self::PatternConstraint<'a, E, A, V>
    where
    E: From<Id>,
    A: From<Id>,
    V: From<Value>,
    for<'b> &'b E: Into<Id>,
    for<'b> &'b A: Into<Id>,
    for<'b> &'b V: Into<Value> {
        PACTTribleSetConstraint::new(e, a, v, self)
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
            let mut set = PACTTribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.add(&Trible{ data: key});
            }
        }
    }
}
