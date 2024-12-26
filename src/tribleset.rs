mod triblesetconstraint;

use triblesetconstraint::*;

use crate::query::TriblePattern;

use crate::patch::{Entry, PATCH};
use crate::query::Variable;
use crate::trible::{
    AEVOrder, AVEOrder, EAVOrder, EVAOrder, Trible, TribleSegmentation, VAEOrder, VEAOrder,
    TRIBLE_LEN,
};
use crate::value::{schemas::genid::GenId, ValueSchema};

use std::iter::{FromIterator, Map};
use std::ops::{Add, AddAssign};

#[derive(Debug, Clone)]
pub struct TribleSet {
    pub eav: PATCH<TRIBLE_LEN, EAVOrder, TribleSegmentation>,
    pub vea: PATCH<TRIBLE_LEN, VEAOrder, TribleSegmentation>,
    pub ave: PATCH<TRIBLE_LEN, AVEOrder, TribleSegmentation>,
    pub vae: PATCH<TRIBLE_LEN, VAEOrder, TribleSegmentation>,
    pub eva: PATCH<TRIBLE_LEN, EVAOrder, TribleSegmentation>,
    pub aev: PATCH<TRIBLE_LEN, AEVOrder, TribleSegmentation>,
}

pub struct TribleSetIterator<'a> {
    inner: Map<crate::patch::PATCHIterator<'a, 64, EAVOrder, TribleSegmentation>, fn([u8; 64]) -> Trible>,
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
        let key = Entry::new(&trible.data);
        self.eav.insert(&key);
        self.eva.insert(&key);
        self.aev.insert(&key);
        self.ave.insert(&key);
        self.vea.insert(&key);
        self.vae.insert(&key);
    }

    pub fn contains(&self, trible: &Trible) -> bool {
        self.eav.has_prefix(&trible.data)
    }

    pub fn iter(&self) -> TribleSetIterator {
        TribleSetIterator {
            inner: self.eav.iter().map(|data| Trible::new_raw(data)),
        }
    }
}

impl PartialEq for TribleSet {
    fn eq(&self, other: &Self) -> bool {
        self.eav == other.eav
    }
}

impl Eq for TribleSet {}

impl AddAssign for TribleSet {
    fn add_assign(&mut self, rhs: Self) {
        self.union(rhs);
    }
}

impl Add for TribleSet {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.union(rhs);
        self
    }
}

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
    type PatternConstraint<'a> = TribleSetConstraint;

    fn pattern<'a, V: ValueSchema>(
        &'a self,
        e: Variable<GenId>,
        a: Variable<GenId>,
        v: Variable<V>,
    ) -> Self::PatternConstraint<'static> {
        TribleSetConstraint::new(e, a, v, self.clone())
    }
}

impl<'a> Iterator for TribleSetIterator<'a> {
    type Item = Trible;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> IntoIterator for &'a TribleSet {
    type Item = Trible;
    type IntoIter = TribleSetIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::literature;
    use crate::prelude::*;

    use super::*;
    use fake::{faker::lorem::en::Words, faker::name::raw::{FirstName, LastName}, locales::EN, Fake};
    
    use rayon::iter::{IntoParallelIterator, ParallelIterator};

    #[test]
    fn union() {
        let mut kb = TribleSet::new();
        for _i in 0..2000 {
            let author = ufoid();
            let book = ufoid();
            kb += literature::entity!(&author, {
                firstname: FirstName(EN).fake::<String>(),
                lastname: LastName(EN).fake::<String>(),
            });
            kb += literature::entity!(&book, {
                title: Words(1..3).fake::<Vec<String>>().join(" "),
                author: &author
            });
        }
        assert_eq!(kb.len(), 8000);
    }

    #[test]
    fn union_parallel() {
        let kb = (0..1000000)
            .into_par_iter()
            .flat_map(|_| {
                let author = ufoid();
                let book = ufoid();
                [
                    literature::entity!(&author, {
                        firstname: FirstName(EN).fake::<String>(),
                        lastname: LastName(EN).fake::<String>(),
                    }),
                    literature::entity!(&book, {
                        title: Words(1..3).fake::<Vec<String>>().join(" "),
                        author: &author
                    }),
                ]
            })
            .reduce(|| TribleSet::new(), |a, b| a + b);
        assert_eq!(kb.len(), 4000000);
    }
/*
    #[test]
    fn intersection() {
        let mut kb1 = TribleSet::new();
        let mut kb2 = TribleSet::new();
        for _i in 0..1000 {
            let author = ufoid();
            let book = ufoid();
            kb1 += literature::entity!(&author, {
                firstname: FirstName(EN).fake::<String>(),
                lastname: LastName(EN).fake::<String>(),
            });
            kb1 += literature::entity!(&book, {
                title: Words(1..3).fake::<Vec<String>>().join(" "),
                author: &author
            });
            kb2 += literature::entity!(&author, {
                firstname: FirstName(EN).fake::<String>(),
                lastname: LastName(EN).fake::<String>(),
            });
            kb2 += literature::entity!(&book, {
                title: Words(1..3).fake::<Vec<String>>().join(" "),
                author: &author
            });
        }
        let intersection = kb1.intersection(&kb2);
        // Verify that the intersection contains only elements present in both kb1 and kb2
        for trible in &intersection {
            assert!(kb1.contains(trible));
            assert!(kb2.contains(trible));
        }
    }

    #[test]
    fn difference() {
        let mut kb1 = TribleSet::new();
        let mut kb2 = TribleSet::new();
        for _i in 0..1000 {
            let author = ufoid();
            let book = ufoid();
            kb1 += literature::entity!(&author, {
                firstname: FirstName(EN).fake::<String>(),
                lastname: LastName(EN).fake::<String>(),
            });
            kb1 += literature::entity!(&book, {
                title: Words(1..3).fake::<Vec<String>>().join(" "),
                author: &author
            });
            if _i % 2 == 0 {
                kb2 += literature::entity!(&author, {
                    firstname: FirstName(EN).fake::<String>(),
                    lastname: LastName(EN).fake::<String>(),
                });
                kb2 += literature::entity!(&book, {
                    title: Words(1..3).fake::<Vec<String>>().join(" "),
                    author: &author
                });
            }
        }
        let difference = kb1.difference(&kb2);
        // Verify that the difference contains only elements present in kb1 but not in kb2
        for trible in &difference {
            assert!(kb1.contains(trible));
            assert!(!kb2.contains(trible));
        }
    }
*/
    #[test]
    fn test_contains() {
        let mut kb = TribleSet::new();
        let author = ufoid();
        let book = ufoid();
        let author_tribles = literature::entity!(&author, {
            firstname: FirstName(EN).fake::<String>(),
            lastname: LastName(EN).fake::<String>(),
        });
        let book_tribles = literature::entity!(&book, {
            title: Words(1..3).fake::<Vec<String>>().join(" "),
            author: &author
        });

        kb += author_tribles.clone();
        kb += book_tribles.clone();

        for trible in &author_tribles {
            assert!(kb.contains(&trible));
        }
        for trible in &book_tribles {
            assert!(kb.contains(&trible));
        }

        let non_existent_trible = literature::entity!(&ufoid(), {
            firstname: FirstName(EN).fake::<String>(),
            lastname: LastName(EN).fake::<String>(),
        });

        for trible in &non_existent_trible {
            assert!(!kb.contains(&trible));
        }
    }
}
