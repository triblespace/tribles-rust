mod tribleindexsetconstraint;

use indexset::BTreeSet;
use tribleindexsetconstraint::*;

use crate::query::TriblePattern;

use crate::query::Variable;
use crate::trible::{
    AEVOrder, AVEOrder, EAVOrder, EVAOrder, AbstractTrible, Trible, VAEOrder, VEAOrder, TRIBLE_LEN
};
use crate::value::{schemas::genid::GenId, ValueSchema};

use std::iter::FromIterator;

#[derive(Debug, Clone)]
pub struct TribleIndexSet {
    pub eav: BTreeSet<AbstractTrible<EAVOrder>>,
    pub vea: BTreeSet<AbstractTrible<VEAOrder>>,
    pub ave: BTreeSet<AbstractTrible<AVEOrder>>,
    pub vae: BTreeSet<AbstractTrible<VAEOrder>>,
    pub eva: BTreeSet<AbstractTrible<EVAOrder>>,
    pub aev: BTreeSet<AbstractTrible<AEVOrder>>,
}

impl TribleIndexSet {
    pub fn union(&mut self, other: TribleIndexSet) {
        self.eav.union(&other.eav);
        self.eva.union(&other.eva);
        self.aev.union(&other.aev);
        self.ave.union(&other.ave);
        self.vea.union(&other.vea);
        self.vae.union(&other.vae);
    }

    pub fn new() -> TribleIndexSet {
        TribleIndexSet {
            eav: BTreeSet::new(),
            eva: BTreeSet::new(),
            aev: BTreeSet::new(),
            ave: BTreeSet::new(),
            vea: BTreeSet::new(),
            vae: BTreeSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        return self.eav.len() as usize;
    }

    pub fn insert(&mut self, trible: &Trible) {
        self.insert_raw(&trible.data)
    }

    pub fn insert_raw(&mut self, data: &[u8; TRIBLE_LEN]) {
        let t = Trible::new_raw(data);
        self.eav.insert(t.reordered());
        self.eva.insert(t.reordered());
        self.aev.insert(t.reordered());
        self.ave.insert(t.reordered());
        self.vea.insert(t.reordered());
        self.vae.insert(t.reordered());
    }
}

impl PartialEq for TribleIndexSet {
    fn eq(&self, other: &Self) -> bool {
        self.eav == other.eav
    }
}

impl Eq for TribleIndexSet {}

impl FromIterator<Trible> for TribleIndexSet {
    fn from_iter<I: IntoIterator<Item = Trible>>(iter: I) -> Self {
        let mut set = TribleIndexSet::new();

        for t in iter {
            set.insert(&t);
        }

        set
    }
}

impl TriblePattern for TribleIndexSet {
    type PatternConstraint<'a> = TribleIndexSetConstraint;

    fn pattern<'a, V: ValueSchema>(
        &'a self,
        e: Variable<GenId>,
        a: Variable<GenId>,
        v: Variable<V>,
    ) -> Self::PatternConstraint<'static> {
        TribleIndexSetConstraint::new(e, a, v, self.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::valueschemas::*;
    use crate::prelude::*;

    use super::*;
    use fake::{faker::name::raw::Name, locales::EN, Fake};
    use itertools::Itertools;
    use proptest::prelude::*;

    NS! {
        pub namespace knights {
            "328edd7583de04e2bedd6bd4fd50e651" as loves: GenId;
            "328147856cc1984f0806dbb824d2b4cb" as name: ShortString;
        }
    }

    #[test]
    fn union() {
        let mut kb = TribleIndexSet::new();
        for _i in 0..2000 {
            let lover_a = ufoid();
            let lover_b = ufoid();
            knights::entity!(&mut kb, &lover_a, {
                name: Name(EN).fake::<String>(),
                loves: &lover_b
            });
            knights::entity!(&mut kb, &lover_b, {
                name: Name(EN).fake::<String>(),
                loves: &lover_a
            });
        }
        assert_eq!(kb.len(), 8000);
    }

    proptest! {
        #[test]
        fn insert(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut set = TribleIndexSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.insert(&Trible{ data: key});
            }
        }
    }
}
