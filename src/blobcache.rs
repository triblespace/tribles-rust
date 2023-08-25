mod tribleconstraint;

use tribleconstraint::*;

use crate::patch::PATCH;
use crate::trible::{AEVOrder, AVEOrder, EAVOrder, EVAOrder, Trible, VAEOrder, VEAOrder};
use std::iter::FromIterator;
use triomphe::Arc;

#[derive(Debug, Clone)]
pub struct BlobCache<const limit:usize> {
    weak: PATCH<32, IdentityOrder>,
    strong: PATCH<64, VAEOrder>,
}

impl BlobCache {
    pub fn union<I>(sets: I) -> TribleSet
    where
        I: IntoIterator<Item = TribleSet>,
        I::IntoIter: Clone,
    {
        let iter = sets.into_iter();
        let eav = PATCH::union(iter.clone().map(|set| set.eav));
        let eva = PATCH::union(iter.clone().map(|set| set.eva));
        let aev = PATCH::union(iter.clone().map(|set| set.aev));
        let ave = PATCH::union(iter.clone().map(|set| set.ave));
        let vea = PATCH::union(iter.clone().map(|set| set.vea));
        let vae = PATCH::union(iter.clone().map(|set| set.vae));

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
            eav: PATCH::new(),
            eva: PATCH::new(),
            aev: PATCH::new(),
            ave: PATCH::new(),
            vea: PATCH::new(),
            vae: PATCH::new(),
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
