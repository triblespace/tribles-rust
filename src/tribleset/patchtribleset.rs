mod patchtribleconstraint;

use patchtribleconstraint::*;

use crate::namespace::{Id, Value};
use crate::patch::{Entry, PATCH};
use crate::trible::{
    AEVOrder, AVEOrder, EAVOrder, EVAOrder, Trible, TribleSegmentation, VAEOrder, VEAOrder,
};
use std::iter::FromIterator;

use super::TribleSet;

#[derive(Debug, Clone)]
pub struct PATCHTribleSet {
    eav: PATCH<64, EAVOrder, TribleSegmentation>,
    eva: PATCH<64, EVAOrder, TribleSegmentation>,
    aev: PATCH<64, AEVOrder, TribleSegmentation>,
    ave: PATCH<64, AVEOrder, TribleSegmentation>,
    vea: PATCH<64, VEAOrder, TribleSegmentation>,
    vae: PATCH<64, VAEOrder, TribleSegmentation>,
}

impl PATCHTribleSet {
    pub fn union<'a, I>(iter: I) -> PATCHTribleSet
    where
        I: Iterator<Item = &'a PATCHTribleSet> + Clone,
    {
        let eav = PATCH::union(iter.clone().map(|set| &set.eav));
        let eva = PATCH::union(iter.clone().map(|set| &set.eva));
        let aev = PATCH::union(iter.clone().map(|set| &set.aev));
        let ave = PATCH::union(iter.clone().map(|set| &set.ave));
        let vea = PATCH::union(iter.clone().map(|set| &set.vea));
        let vae = PATCH::union(iter.clone().map(|set| &set.vae));

        PATCHTribleSet {
            eav,
            eva,
            aev,
            ave,
            vea,
            vae,
        }
    }

    pub fn new() -> PATCHTribleSet {
        PATCHTribleSet {
            eav: PATCH::new(),
            eva: PATCH::new(),
            aev: PATCH::new(),
            ave: PATCH::new(),
            vea: PATCH::new(),
            vae: PATCH::new(),
        }
    }

    pub fn len(&self) -> u64 {
        return self.eav.len();
    }

    pub fn add(&mut self, trible: &Trible) {
        let key = Entry::new(&trible.data);
        self.eav.put(&key);
        self.eva.put(&key);
        self.aev.put(&key);
        self.ave.put(&key);
        self.vea.put(&key);
        self.vae.put(&key);
    }
}

impl FromIterator<Trible> for PATCHTribleSet {
    fn from_iter<I: IntoIterator<Item = Trible>>(iter: I) -> Self {
        let mut set = PATCHTribleSet::new();

        for t in iter {
            set.add(&t);
        }

        set
    }
}

impl TribleSet for PATCHTribleSet {
    type PatternConstraint<'a, E, A, V>
     = PATCHTribleSetConstraint<'a, E, A, V>
     where
     E: From<Id>,
     A: From<Id>,
     V: From<Value>,
     for<'b> &'b E: Into<Id>,
     for<'b> &'b A: Into<Id>,
     for<'b> &'b V: Into<Value>;

    fn pattern<'a, E, A, V>(
        &'a self,
        e: crate::query::Variable<E>,
        a: crate::query::Variable<A>,
        v: crate::query::Variable<V>,
    ) -> Self::PatternConstraint<'a, E, A, V>
    where
        E: From<Id>,
        A: From<Id>,
        V: From<Value>,
        for<'b> &'b E: Into<Id>,
        for<'b> &'b A: Into<Id>,
        for<'b> &'b V: Into<Value>,
    {
        PATCHTribleSetConstraint::new(e, a, v, self)
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
            let mut set = PATCHTribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.add(&Trible{ data: key});
            }
        }
    }
}