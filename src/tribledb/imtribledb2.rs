use crate::trible::*;
use crate::tribledb::query::*;
use crate::tribledb::TribleDB;
use im_rc::OrdSet;
use std::cmp::Ordering;

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

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct EAVTrible(pub Trible);
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct EVATrible(pub Trible);
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct AEVTrible(pub Trible);
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct AVETrible(pub Trible);
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct VEATrible(pub Trible);
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct VAETrible(pub Trible);

impl Ord for EAVTrible {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.e, self.0.a, self.0.v1, self.0.v2)
            .cmp(&(other.0.e, other.0.a, other.0.v1, other.0.v2))
    }
}

impl PartialOrd for EAVTrible {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            (self.0.e, self.0.a, self.0.v1, self.0.v2)
                .cmp(&(other.0.e, other.0.a, other.0.v1, other.0.v2)),
        )
    }
}

impl Ord for EVATrible {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.e, self.0.v1, self.0.v2, self.0.a)
            .cmp(&(other.0.e, other.0.v1, other.0.v2, other.0.a))
    }
}

impl PartialOrd for EVATrible {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            (self.0.e, self.0.v1, self.0.v2, self.0.a)
                .cmp(&(other.0.e, other.0.v1, other.0.v2, other.0.a)),
        )
    }
}

impl Ord for AEVTrible {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.a, self.0.e, self.0.v1, self.0.v2)
            .cmp(&(other.0.a, other.0.e, other.0.v1, other.0.v2))
    }
}

impl PartialOrd for AEVTrible {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            (self.0.a, self.0.e, self.0.v1, self.0.v2)
                .cmp(&(other.0.a, other.0.e, other.0.v1, other.0.v2)),
        )
    }
}

impl Ord for AVETrible {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.a, self.0.v1, self.0.v2, self.0.e)
            .cmp(&(other.0.a, other.0.v1, other.0.v2, other.0.e))
    }
}

impl PartialOrd for AVETrible {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            (self.0.a, self.0.v1, self.0.v2, self.0.e)
                .cmp(&(other.0.a, other.0.v1, other.0.v2, other.0.e)),
        )
    }
}

impl Ord for VEATrible {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.v1, self.0.v2, self.0.e, self.0.a)
            .cmp(&(other.0.v1, other.0.v2, other.0.e, other.0.a))
    }
}

impl PartialOrd for VEATrible {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            (self.0.v1, self.0.v2, self.0.e, self.0.a)
                .cmp(&(other.0.v1, other.0.v2, other.0.e, other.0.a)),
        )
    }
}

impl Ord for VAETrible {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.v1, self.0.v2, self.0.e, self.0.a)
            .cmp(&(other.0.v1, other.0.v2, other.0.e, other.0.a))
    }
}

impl PartialOrd for VAETrible {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            (self.0.v1, self.0.v2, self.0.a, self.0.e)
                .cmp(&(other.0.v1, other.0.v2, other.0.a, other.0.e)),
        )
    }
}

#[derive(Clone)]
pub struct ImTribleDB2 {
    e_a_v1_v2: OrdSet<EAVTrible>,
    e_v1_v2_a: OrdSet<EVATrible>,
    a_e_v1_v2: OrdSet<AEVTrible>,
    a_v1_v2_e: OrdSet<AVETrible>,
    v1_v2_e_a: OrdSet<VEATrible>,
    v1_v2_a_e: OrdSet<VAETrible>,
    ea: OrdSet<Segment>,
    ev1: OrdSet<Segment>,
    av1: OrdSet<Segment>,
    eav1: OrdSet<Segment>,
}

impl Default for ImTribleDB2 {
    fn default() -> Self {
        ImTribleDB2 {
            e_a_v1_v2: OrdSet::new(),
            e_v1_v2_a: OrdSet::new(),
            a_e_v1_v2: OrdSet::new(),
            a_v1_v2_e: OrdSet::new(),
            v1_v2_e_a: OrdSet::new(),
            v1_v2_a_e: OrdSet::new(),
            ea: OrdSet::new(),
            ev1: OrdSet::new(),
            av1: OrdSet::new(),
            eav1: OrdSet::new(),
        }
    }
}

impl TribleDB for ImTribleDB2 {
    fn with<'a, T>(&self, tribles: T) -> ImTribleDB2
    where
        T: Iterator<Item = &'a Trible> + Clone,
    {
        let mut db = self.clone();
        for trible in tribles.clone() {
            db.e_a_v1_v2 = db.e_a_v1_v2.update(EAVTrible(*trible));
        }
        for trible in tribles.clone() {
            db.e_v1_v2_a = db.e_v1_v2_a.update(EVATrible(*trible));
        }
        for trible in tribles.clone() {
            db.a_e_v1_v2 = db.a_e_v1_v2.update(AEVTrible(*trible));
        }
        for trible in tribles.clone() {
            db.a_v1_v2_e = db.a_v1_v2_e.update(AVETrible(*trible));
        }
        for trible in tribles.clone() {
            db.v1_v2_e_a = db.v1_v2_e_a.update(VEATrible(*trible));
        }
        for trible in tribles.clone() {
            db.v1_v2_a_e = db.v1_v2_a_e.update(VAETrible(*trible));
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
