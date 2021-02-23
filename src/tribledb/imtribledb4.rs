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

fn scrambleEAV(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(trible.e.0, trible.a.0, trible.v1.0, trible.v2.0)
}

fn scrambleEVA(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(trible.e.0, trible.v1.0, trible.v2.0, trible.a.0)
}

fn scrambleAEV(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(trible.a.0, trible.e.0, trible.v1.0, trible.v2.0)
}

fn scrambleAVE(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(trible.a.0, trible.v1.0, trible.v2.0, trible.e.0)
}

fn scrambleVEA(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(trible.v1.0, trible.v2.0, trible.e.0, trible.a.0)
}

fn scrambleVAE(trible: &Trible) -> ScrambledTrible {
    ScrambledTrible(trible.v1.0, trible.v2.0, trible.a.0, trible.e.0)
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScrambledTrible(Segment, Segment, Segment, Segment);

#[derive(Clone)]
pub struct ImTribleDB4 {
    pool: OrdSetPool<ScrambledTrible>,
    e_a_v1_v2: OrdSet<ScrambledTrible>,
    e_v1_v2_a: OrdSet<ScrambledTrible>,
    a_e_v1_v2: OrdSet<ScrambledTrible>,
    a_v1_v2_e: OrdSet<ScrambledTrible>,
    v1_v2_e_a: OrdSet<ScrambledTrible>,
    v1_v2_a_e: OrdSet<ScrambledTrible>,
    ea: OrdSet<Segment>,
    ev1: OrdSet<Segment>,
    av1: OrdSet<Segment>,
    eav1: OrdSet<Segment>,
}

impl Default for ImTribleDB4 {
    fn default() -> Self {
        let pool: OrdSetPool<ScrambledTrible> = OrdSetPool::new(10000000);
        ImTribleDB4 {
            pool: pool.clone(),
            e_a_v1_v2: OrdSet::with_pool(&pool),
            e_v1_v2_a: OrdSet::with_pool(&pool),
            a_e_v1_v2: OrdSet::with_pool(&pool),
            a_v1_v2_e: OrdSet::with_pool(&pool),
            v1_v2_e_a: OrdSet::with_pool(&pool),
            v1_v2_a_e: OrdSet::with_pool(&pool),
            ea: OrdSet::new(),
            ev1: OrdSet::new(),
            av1: OrdSet::new(),
            eav1: OrdSet::new(),
        }
    }
}

impl TribleDB for ImTribleDB4 {
    fn with<'a, T>(&self, tribles: T) -> ImTribleDB4
    where
        T: Iterator<Item = &'a Trible> + Clone,
    {
        let mut db = self.clone();
        for trible in tribles.clone() {
            db.e_a_v1_v2 = db.e_a_v1_v2.update(scrambleEAV(trible));
        }
        for trible in tribles.clone() {
            db.e_v1_v2_a = db.e_v1_v2_a.update(scrambleEVA(trible));
        }
        for trible in tribles.clone() {
            db.a_e_v1_v2 = db.a_e_v1_v2.update(scrambleAEV(trible));
        }
        for trible in tribles.clone() {
            db.a_v1_v2_e = db.a_v1_v2_e.update(scrambleAVE(trible));
        }
        for trible in tribles.clone() {
            db.v1_v2_e_a = db.v1_v2_e_a.update(scrambleVEA(trible));
        }
        for trible in tribles.clone() {
            db.v1_v2_a_e = db.v1_v2_a_e.update(scrambleVAE(trible));
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
