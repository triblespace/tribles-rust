use crate::trible::*;
use crate::tribledb::query::*;
use crate::tribledb::TribleDB;
use im_rc::OrdSet;
use im_rc::ordset::OrdSetPool;

/*
Note, [x x y] can be seen as a compound index key. E=A ->V
Searchign for both E and A at once.

> E -> A -> V1 -> V2
    -> V1 -> V2 -> A
> A -> E -> V1 -> V2
    -> V1 -> V2 -> E
> V1 -> V2 -> E -> A
           -> A -> E
> E=A
> A=V
> E=V
> E=A=V

*/

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Index {
    EAV,
    EVA,
    AEV,
    AVE,
    VEA,
    VAE
}

fn scrambleEAV(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(Index::EAV, trible.e.0, trible.a.0, trible.v1.0, trible.v2.0)
}

fn scrambleEVA(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(Index::EVA, trible.e.0, trible.v1.0, trible.v2.0, trible.a.0)
}

fn scrambleAEV(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(Index::AEV, trible.a.0, trible.e.0, trible.v1.0, trible.v2.0)
}

fn scrambleAVE(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(Index::AVE, trible.a.0, trible.v1.0, trible.v2.0, trible.e.0)
}

fn scrambleVEA(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(Index::VEA, trible.v1.0, trible.v2.0, trible.e.0, trible.a.0)
}

fn scrambleVAE(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(Index::VAE, trible.v1.0, trible.v2.0, trible.a.0, trible.e.0)
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScrambledTrible(Index, Segment, Segment, Segment, Segment);

#[derive(Clone)]
pub struct ImTribleDB5 {
    pool: OrdSetPool<ScrambledTrible>,
    index: OrdSet<ScrambledTrible>,
    ea: OrdSet<Segment>,
    ev1: OrdSet<Segment>,
    av1: OrdSet<Segment>,
    eav1: OrdSet<Segment>,
}

impl Default for ImTribleDB5 {
    fn default() -> Self {
        let pool: OrdSetPool<ScrambledTrible> = OrdSetPool::new(10000000);
        ImTribleDB5 {
            pool: pool.clone(),
            index: OrdSet::with_pool(&pool),
            ea: OrdSet::new(),
            ev1: OrdSet::new(),
            av1: OrdSet::new(),
            eav1: OrdSet::new(),
        }
    }
}

impl TribleDB for ImTribleDB5 {
    fn with<'a, T>(&self, tribles: T) -> ImTribleDB5
    where
        T: Iterator<Item = &'a Trible> + Clone,
    {
        let mut db = self.clone();
        for trible in tribles.clone() {
            db.index = db.index.update(scrambleEAV(trible))
                               .update(scrambleEVA(trible))
                               .update(scrambleAEV(trible))
                               .update(scrambleAVE(trible))
                               .update(scrambleVEA(trible))
                               .update(scrambleVAE(trible));
        }

        for trible in tribles.clone() {
            if trible.e.0 == trible.a.0 {
                db.ea = db.ea.update(trible.e.0);
            }

            if trible.a.0 == trible.v1.0 {
                db.av1 = db.av1.update(trible.a.0);
            }

            if trible.e.0 == trible.v1.0 {
                db.ev1 = db.ev1.update(trible.e.0);
            }

            if trible.e.0 == trible.a.0 && trible.a.0 == trible.v1.0 {
                db.eav1 = db.eav1.update(trible.e.0);
            }
        }

        return db;
    }
}
