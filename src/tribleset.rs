mod triblesetconstraint;

use triblesetconstraint::*;

use crate::query::TriblePattern;

use crate::patch::{Entry, PATCH};
use crate::trible::{
    AEVOrder, AVEOrder, EAVOrder, EVAOrder, Trible, TribleSegmentation, VAEOrder, VEAOrder,
    TRIBLE_LEN,
};
use crate::{Id, Value, Valuelike};
use std::iter::FromIterator;

#[derive(Debug, Clone)]
pub struct TribleSet {
    pub eav: PATCH<64, EAVOrder, TribleSegmentation>,
    pub vea: PATCH<64, VEAOrder, TribleSegmentation>,
    pub ave: PATCH<64, AVEOrder, TribleSegmentation>,
    pub vae: PATCH<64, VAEOrder, TribleSegmentation>,
    pub eva: PATCH<64, EVAOrder, TribleSegmentation>,
    pub aev: PATCH<64, AEVOrder, TribleSegmentation>,
}

impl TribleSet {
    pub fn union(&mut self, other: Self) {
        self.eav.union(other.eav);
        self.eva.union(other.eva);
        self.aev.union(other.aev);
        self.ave.union(other.ave);
        self.vea.union(other.vea);
        self.vae.union(other.vae);
    }

    pub fn new() -> TribleSet {
        TribleSet {
            eav: PATCH::new(),
            eva: PATCH::new(),
            aev: PATCH::new(),
            ave: PATCH::new(),
            vea: PATCH::new(),
            vae: PATCH::new(),
        }
    }

    pub fn len(&self) -> usize {
        return self.eav.len() as usize;
    }

    pub fn insert(&mut self, trible: &Trible) {
        self.insert_raw(&trible.data)
    }

    pub fn insert_raw(&mut self, data: &[u8; TRIBLE_LEN]) {
        let key = Entry::new(data);
        self.eav.insert(&key);
        self.eva.insert(&key);
        self.aev.insert(&key);
        self.ave.insert(&key);
        self.vea.insert(&key);
        self.vae.insert(&key);
    }
}

impl PartialEq for TribleSet {
    fn eq(&self, other: &Self) -> bool {
        self.eav == other.eav
    }
}

impl Eq for TribleSet {}

impl FromIterator<Trible> for TribleSet {
    fn from_iter<I: IntoIterator<Item = Trible>>(iter: I) -> Self {
        let mut set = TribleSet::new();

        for t in iter {
            set.insert(&t);
        }

        set
    }
}

impl TriblePattern for TribleSet {
    type PatternConstraint<'a, V>
     = TribleSetConstraint<'a, V>
     where V: Valuelike;

    fn pattern<'a, V>(
        &'a self,
        e: crate::query::Variable<Id>,
        a: crate::query::Variable<Id>,
        v: crate::query::Variable<V>,
    ) -> Self::PatternConstraint<'a, V>
    where
        V: Valuelike,
    {
        TribleSetConstraint::new(e, a, v, self)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use crate::{types::SmallString, ufoid, Id, NS};

    use super::*;
    use fake::{faker::name::raw::Name, locales::EN, Fake};
    use itertools::Itertools;
    use proptest::prelude::*;

    NS! {
        pub namespace knights {
            "328edd7583de04e2bedd6bd4fd50e651" as loves: Id;
            "328147856cc1984f0806dbb824d2b4cb" as name: SmallString;
        }
    }

    #[test]
    fn union() {
        let mut kb = TribleSet::new();
        for _i in 0..2000 {
            let lover_a = ufoid();
            let lover_b = ufoid();
            kb.union(knights::entity!(lover_a, {
                name: (&Name(EN).fake::<String>()[..]).try_into().unwrap(),
                loves: lover_b
            }));
            kb.union(knights::entity!(lover_b, {
                name: (&Name(EN).fake::<String>()[..]).try_into().unwrap(),
                loves: lover_a
            }));
        }
        assert_eq!(kb.len(), 8000);
    }

    proptest! {
        #[test]
        fn insert(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut set = TribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.insert(&Trible{ data: key});
            }
        }
    }
}
