use crate::trible::*;
use crate::tribledb::query::*;
use crate::tribledb::TribleDB;
use im_rc::ordmap::OrdMapPool;
use im_rc::OrdMap;
use im_rc::OrdSet;

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

type IndexSegment = OrdMap<Segment, Branch>;

#[derive(Clone)]
enum Branch {
    Index(IndexSegment, IndexSegment, IndexSegment),
    E(IndexSegment, IndexSegment),
    EA(IndexSegment),
    EAV1(IndexSegment),
    EAV1V2(),
    EV1(IndexSegment),
    EV1V2(IndexSegment),
    EV1V2A(),
    A(IndexSegment, IndexSegment),
    AE(IndexSegment),
    AEV1(IndexSegment),
    AEV1V2(),
    AV1(IndexSegment),
    AV1V2(IndexSegment),
    AV1V2E(),
    V1(IndexSegment),
    V1V2(IndexSegment, IndexSegment),
    V1V2A(IndexSegment),
    V1V2AE(),
    V1V2E(IndexSegment),
    V1V2EA(),
}

pub fn upsert<K, V, F>(m: &OrdMap<K, V>, k: K, f: F) -> OrdMap<K, V>
where
    F: FnOnce(Option<&V>) -> V,
    K: Ord + Copy,
    V: Clone,
{
    let mut out = m.clone();
    out.insert(k, f(m.get(&k)));
    out
}

fn branch_with(pool: &OrdMapPool<Segment, Branch>, branch: &Branch, trible: &Trible) -> Branch {
    let Trible {
        e: E(te),
        a: A(ta),
        v1: V1(tv1),
        v2: V2(tv2),
    } = *trible;
    match branch {
        Branch::Index(e, a, v1) => Branch::Index(
            upsert(e, te, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(
                        pool,
                        &Branch::E(OrdMap::with_pool(pool), OrdMap::with_pool(pool)),
                        trible,
                    )
                }
            }),
            upsert(a, ta, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(
                        pool,
                        &Branch::A(OrdMap::with_pool(pool), OrdMap::with_pool(pool)),
                        trible,
                    )
                }
            }),
            upsert(v1, tv1, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(pool, &Branch::V1(OrdMap::with_pool(pool)), trible)
                }
            }),
        ),
        Branch::E(a, v1) => Branch::E(
            upsert(a, ta, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(pool, &Branch::EA(OrdMap::with_pool(pool)), trible)
                }
            }),
            upsert(v1, tv1, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(pool, &Branch::EV1(OrdMap::with_pool(pool)), trible)
                }
            }),
        ),
        Branch::EA(v1) => Branch::EA(upsert(v1, tv1, |may| {
            if let Some(b) = may {
                branch_with(pool, b, trible)
            } else {
                branch_with(pool, &Branch::EAV1(OrdMap::with_pool(pool)), trible)
            }
        })),
        Branch::EAV1(v2) => Branch::EAV1(v2.update(tv2, Branch::EAV1V2())),
        Branch::EV1(v2) => Branch::EV1(upsert(v2, tv2, |may| {
            if let Some(b) = may {
                branch_with(pool, b, trible)
            } else {
                branch_with(pool, &Branch::EV1V2(OrdMap::with_pool(pool)), trible)
            }
        })),
        Branch::EV1V2(a) => Branch::EV1V2(a.update(ta, Branch::EV1V2A())),
        Branch::A(e, v1) => Branch::A(
            upsert(e, te, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(pool, &Branch::AE(OrdMap::with_pool(pool)), trible)
                }
            }),
            upsert(v1, tv1, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(pool, &Branch::AV1(OrdMap::with_pool(pool)), trible)
                }
            }),
        ),
        Branch::AE(v1) => Branch::AE(upsert(v1, tv1, |may| {
            if let Some(b) = may {
                branch_with(pool, b, trible)
            } else {
                branch_with(pool, &Branch::AEV1(OrdMap::with_pool(pool)), trible)
            }
        })),
        Branch::AEV1(v2) => Branch::AEV1(v2.update(tv2, Branch::AEV1V2())),
        Branch::AV1(v2) => Branch::AV1(upsert(v2, tv2, |may| {
            if let Some(b) = may {
                branch_with(pool, b, trible)
            } else {
                branch_with(pool, &Branch::AV1V2(OrdMap::with_pool(pool)), trible)
            }
        })),
        Branch::AV1V2(e) => Branch::AV1V2(e.update(te, Branch::AV1V2E())),
        Branch::V1(v2) => Branch::V1(upsert(v2, tv2, |may| {
            if let Some(b) = may {
                branch_with(pool, b, trible)
            } else {
                branch_with(
                    pool,
                    &Branch::V1V2(OrdMap::with_pool(pool), OrdMap::with_pool(pool)),
                    trible,
                )
            }
        })),
        Branch::V1V2(e, a) => Branch::V1V2(
            upsert(e, te, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(pool, &Branch::V1V2E(OrdMap::with_pool(pool)), trible)
                }
            }),
            upsert(a, ta, |may| {
                if let Some(b) = may {
                    branch_with(pool, b, trible)
                } else {
                    branch_with(pool, &Branch::V1V2A(OrdMap::with_pool(pool)), trible)
                }
            }),
        ),
        Branch::V1V2A(e) => Branch::V1V2A(e.update(te, Branch::V1V2AE())),
        Branch::V1V2E(a) => Branch::V1V2E(a.update(ta, Branch::V1V2EA())),
        _ => branch.clone(),
    }
}

#[derive(Clone)]
pub struct ImTribleDB3 {
    pool: OrdMapPool<Segment, Branch>,
    index: Branch,
    ea: OrdSet<Segment>,
    ev1: OrdSet<Segment>,
    av1: OrdSet<Segment>,
    eav1: OrdSet<Segment>,
}

impl Default for ImTribleDB3 {
    fn default() -> Self {
        let pool = OrdMapPool::new(1000000);
        ImTribleDB3 {
            pool: pool.clone(),
            index: Branch::Index(
                OrdMap::with_pool(&pool),
                OrdMap::with_pool(&pool),
                OrdMap::with_pool(&pool),
            ),
            ea: OrdSet::new(),
            ev1: OrdSet::new(),
            av1: OrdSet::new(),
            eav1: OrdSet::new(),
        }
    }
}

impl TribleDB for ImTribleDB3 {
    fn with<'a, T>(&self, tribles: T) -> ImTribleDB3
    where
        T: Iterator<Item = &'a Trible> + Clone,
    {
        let mut index = self.index.clone();
        let mut ea = self.ea.clone();
        let mut av1 = self.av1.clone();
        let mut ev1 = self.ev1.clone();
        let mut eav1 = self.eav1.clone();
        for trible in tribles {
            index = branch_with(&self.pool, &index, trible);

            if trible.e.0 == trible.a.0 {
                ea = ea.update(trible.e.0);
            }

            if trible.a.0 == trible.v1.0 {
                av1 = av1.update(trible.a.0);
            }

            if trible.e.0 == trible.v1.0 {
                ev1 = ev1.update(trible.e.0);
            }

            if trible.e.0 == trible.a.0 && trible.a.0 == trible.v1.0 {
                eav1 = eav1.update(trible.e.0);
            }
        }

        return ImTribleDB3 {
            pool: self.pool.clone(),
            index,
            ea,
            av1,
            ev1,
            eav1,
        };
    }
    /*
        fn empty(&self) -> Self;
        fn isEmpty(&self) -> bool;
        fn isEqual(&self. other: &Self) -> bool;
        fn isSubsetOf(&self. other: &Self) -> bool;
        fn isProperSubsetOf(&self. other: &Self) -> bool;
        fn isIntersecting(&self. other: &Self) -> bool;
        fn union(&self. other: &Self) -> Self;
        fn subtract(&self. other: &Self) -> Self;
        fn difference(&self. other: &Self) -> Self;
        fn intersect(&self. other: &Self) -> Self;

        fn inner_constraint(
            &self,
            variable: Variable,
            e: bool,
            a: bool,
            v1: bool,
        ) -> Box<dyn Constraint> {
            let index = match (e, a, v1) {
                (true, true, false) => self.index.ea.clone(),
                (false, true, true) => self.index.av1.clone(),
                (true, false, true) => self.index.ev1.clone(),
                (true, true, true) => self.index.eav1.clone(),
                _ => panic!("Bad inner constraint, must select multiple segments."),
            };

            return Box::new(IndexConstraint {
                index: index,
                variable,
                cursor: 0,
                valid: false,
                ascending: true,
            });
        }
        fn trible_constraint(
            &self,
            e: Variable,
            a: Variable,
            v1: Variable,
            v2: Variable,
        ) -> Box<dyn Constraint> {
            return Box::new(TribleConstraint {
                variable_e: e,
                variable_a: a,
                variable_v1: v1,
                variable_v2: v2,
                cursors: vec![TribleCursor::Root(self.index.clone())],
                valid: false,
            });
        }
    }

    pub struct IndexConstraint {
        index: OrdSet<Segment>,
        variable: Variable,
        cursor: Segment,
        valid: bool,
        ascending: bool,
    }

    impl Constraint for IndexConstraint {
        fn propose(&self) -> VariableProposal {
            return VariableProposal {
                variable: self.variable,
                count: self.index.len(),
                forced: false,
            };
        }

        fn push(&mut self, variable: Variable, ascending: bool) -> PushResult {
            if variable != self.variable {
                return PushResult {
                    relevant: false,
                    done: false,
                };
            }
            self.ascending = ascending;
            if ascending {
                match self.index.get_next(&Segment::MIN) {
                    Some(value) => {
                        self.cursor = *value;
                        self.valid = true;
                    }
                    None => {
                        self.valid = false;
                    }
                }
            } else {
                match self.index.get_next(&Segment::MAX) {
                    Some(value) => {
                        self.cursor = *value;
                        self.valid = true;
                    }
                    None => {
                        self.valid = false;
                    }
                }
            }
            return PushResult {
                relevant: true,
                done: true,
            };
        }

        fn pop(&mut self) {
            self.valid = false;
        }
        fn valid(&self) -> bool {
            return self.valid;
        }
        fn peek(&self) -> Segment {
            return self.cursor;
        }
        fn next(&mut self) {
            if self.ascending {
                if self.cursor == Segment::MAX {
                    self.valid = false;
                    return;
                }

                match self.index.get_next(&self.cursor) {
                    Some(value) => {
                        self.cursor = *value;
                    }
                    None => {
                        self.valid = false;
                    }
                }
            } else {
                if self.cursor == Segment::MIN {
                    self.valid = false;
                    return;
                }

                match self.index.get_prev(&self.cursor) {
                    Some(value) => {
                        self.cursor = *value;
                    }
                    None => {
                        self.valid = false;
                    }
                }
            }
        }
        fn seek(&mut self, value: Segment) -> bool {
            if self.ascending {
                if self.cursor == Segment::MAX {
                    self.valid = false;
                    return false;
                }

                match self.index.get_next(&self.cursor) {
                    Some(found_value) => {
                        if value == *found_value {
                            self.cursor = *found_value;
                            return true;
                        } else {
                            self.cursor = *found_value;
                            return false;
                        }
                    }
                    None => {
                        self.valid = false;
                        return false;
                    }
                }
            } else {
                if self.cursor == Segment::MIN {
                    self.valid = false;
                    return false;
                }
                match self.index.get_prev(&self.cursor) {
                    Some(found_value) => {
                        if value == *found_value {
                            self.cursor = *found_value;
                            return true;
                        } else {
                            self.cursor = *found_value;
                            return false;
                        }
                    }
                    None => {
                        self.valid = false;
                        return false;
                    }
                }
            }
        }
    }

    enum TribleCursor {
        Root(IndexBranch),
        E(OrdMap<E, EBranch>, E, bool),
        A(OrdMap<A, ABranch>, A, bool),
        V1(OrdMap<V1, V1Branch>, V1, bool),
        V1V2(OrdMap<V2, V1V2Branch>, V2, bool),
        EA(OrdMap<A, EABranch>, A, bool),
        EAV1(OrdMap<V1, EAV1Branch>, V1, bool),
        EAV1V2(OrdSet<V2>, V2, bool),
        EV1(OrdMap<V1, EV1Branch>, V1, bool),
        EV1V2(OrdMap<V2, EV1V2Branch>, V2, bool),
        EV1V2A(OrdSet<A>, A, bool),
        AE(OrdMap<E, AEBranch>, E, bool),
        AEV1(OrdMap<V1, AEV1Branch>, V1, bool),
        AEV1V2(OrdSet<V2>, V2, bool),
        AV1(OrdMap<V1, AV1Branch>, V1, bool),
        AV1V2(OrdMap<V2, AV1V2Branch>, V2, bool),
        AV1V2E(OrdSet<E>, E, bool),
        V1V2E(OrdMap<E, V1V2EBranch>, E, bool),
        V1V2EA(OrdSet<A>, A, bool),
        V1V2A(OrdMap<A, V1V2ABranch>, A, bool),
        V1V2AE(OrdSet<E>, E, bool),
        Invalid(),
    }

    pub struct TribleConstraint {
        variable_e: Variable,
        variable_a: Variable,
        variable_v1: Variable,
        variable_v2: Variable,
        cursors: Vec<TribleCursor>,
        valid: bool,
    }

    impl Constraint for TribleConstraint {
        fn propose(&self) -> VariableProposal {
            match self.cursors.last().unwrap() {
                TribleCursor::Root(IndexBranch { e, a, v1, .. }) => {
                    let mut count = e.len();
                    let mut variable = self.variable_e;
                    if a.len() < count {
                        count = a.len();
                        variable = self.variable_a;
                    }
                    if v1.len() < count {
                        count = v1.len();
                        variable = self.variable_v1;
                    }
                    return VariableProposal {
                        variable,
                        count,
                        forced: false,
                    };
                }
                TribleCursor::E(index, value, _) => {
                    let EBranch { a, v1 } = index.get(value).unwrap();
                    if a.len() < v1.len() {
                        return VariableProposal {
                            variable: self.variable_a,
                            count: a.len(),
                            forced: false,
                        };
                    } else {
                        return VariableProposal {
                            variable: self.variable_v1,
                            count: v1.len(),
                            forced: false,
                        };
                    }
                }
                TribleCursor::A(index, value, _) => {
                    let ABranch { e, v1 } = index.get(value).unwrap();
                    if e.len() < v1.len() {
                        return VariableProposal {
                            variable: self.variable_e,
                            count: e.len(),
                            forced: false,
                        };
                    } else {
                        return VariableProposal {
                            variable: self.variable_v1,
                            count: v1.len(),
                            forced: false,
                        };
                    }
                }
                TribleCursor::V1(index, value, _) => {
                    let V1Branch { v2 } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_v2,
                        count: v2.len(),
                        forced: true,
                    };
                }
                TribleCursor::V1V2(index, value, _) => {
                    let V1V2Branch { e, a } = index.get(value).unwrap();
                    if e.len() < a.len() {
                        return VariableProposal {
                            variable: self.variable_e,
                            count: e.len(),
                            forced: false,
                        };
                    } else {
                        return VariableProposal {
                            variable: self.variable_a,
                            count: a.len(),
                            forced: false,
                        };
                    }
                }
                TribleCursor::EA(index, value, _) => {
                    let EABranch { v1 } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_v1,
                        count: v1.len(),
                        forced: false,
                    };
                }
                TribleCursor::EAV1(index, value, _) => {
                    let EAV1Branch { v2 } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_v2,
                        count: v2.len(),
                        forced: true,
                    };
                }
                TribleCursor::EV1(index, value, _) => {
                    let EV1Branch { v2 } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_v2,
                        count: v2.len(),
                        forced: true,
                    };
                }
                TribleCursor::EV1V2(index, value, _) => {
                    let EV1V2Branch { a } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_a,
                        count: a.len(),
                        forced: false,
                    };
                }
                TribleCursor::AE(index, value, _) => {
                    let AEBranch { v1 } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_v1,
                        count: v1.len(),
                        forced: false,
                    };
                }
                TribleCursor::AEV1(index, value, _) => {
                    let AEV1Branch { v2 } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_v2,
                        count: v2.len(),
                        forced: true,
                    };
                }
                TribleCursor::AV1(index, value, _) => {
                    let AV1Branch { v2 } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_v2,
                        count: v2.len(),
                        forced: true,
                    };
                }
                TribleCursor::AV1V2(index, value, _) => {
                    let AV1V2Branch { e } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_e,
                        count: e.len(),
                        forced: false,
                    };
                }
                TribleCursor::V1V2E(index, value, _) => {
                    let V1V2EBranch { a } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_a,
                        count: a.len(),
                        forced: true,
                    };
                }
                TribleCursor::V1V2A(index, value, _) => {
                    let V1V2ABranch { e } = index.get(value).unwrap();
                    return VariableProposal {
                        variable: self.variable_e,
                        count: e.len(),
                        forced: true,
                    };
                }
                TribleCursor::EAV1V2(..)
                | TribleCursor::EV1V2A(..)
                | TribleCursor::AEV1V2(..)
                | TribleCursor::AV1V2E(..)
                | TribleCursor::V1V2EA(..)
                | TribleCursor::V1V2AE(..)
                | TribleCursor::Invalid() => panic!("Should not have been reached for proposing!"),
            }
        }

        fn push(&mut self, variable: Variable, ascending: bool) -> PushResult {
            if variable != self.variable_e
                && variable != self.variable_a
                && variable != self.variable_v1
                && variable != self.variable_v2
            {
                return PushResult {
                    relevant: false,
                    done: false,
                };
            }

            let mut cursor = TribleCursor::Invalid();
            let mut done = false;
            let mut valid = false;

            match self.cursors.last().unwrap() {
                TribleCursor::Root(IndexBranch { e, a, v1, .. }) => {
                    if variable == self.variable_e {
                        if let Some((key, _)) = if ascending {
                            e.get_next(&E(Segment::MIN))
                        } else {
                            e.get_prev(&E(Segment::MAX))
                        } {
                            cursor = TribleCursor::E(e.clone(), *key, ascending);
                            valid = true;
                        }
                    } else if variable == self.variable_a {
                        if let Some((key, _)) = if ascending {
                            a.get_next(&A(Segment::MIN))
                        } else {
                            a.get_prev(&A(Segment::MAX))
                        } {
                            cursor = TribleCursor::A(a.clone(), *key, ascending);
                            valid = true;
                        }
                    } else if variable == self.variable_v1 {
                        if let Some((key, _)) = if ascending {
                            v1.get_next(&V1(Segment::MIN))
                        } else {
                            v1.get_prev(&V1(Segment::MAX))
                        } {
                            cursor = TribleCursor::V1(v1.clone(), *key, ascending);
                            valid = true;
                        }
                    }
                }
                TribleCursor::E(index, value, _) => {
                    let EBranch { a, v1 } = index.get(value).unwrap();
                    if variable == self.variable_a {
                        if let Some((key, _)) = if ascending {
                            a.get_next(&A(Segment::MIN))
                        } else {
                            a.get_prev(&A(Segment::MAX))
                        } {
                            cursor = TribleCursor::EA(a.clone(), *key, ascending);
                            valid = true;
                        }
                    } else if variable == self.variable_v1 {
                        if let Some((key, _)) = if ascending {
                            v1.get_next(&V1(Segment::MIN))
                        } else {
                            v1.get_prev(&V1(Segment::MAX))
                        } {
                            cursor = TribleCursor::EV1(v1.clone(), *key, ascending);
                            valid = true;
                        }
                    }
                }
                TribleCursor::A(index, value, _) => {
                    let ABranch { e, v1 } = index.get(value).unwrap();
                    if variable == self.variable_e {
                        if let Some((key, _)) = if ascending {
                            e.get_next(&E(Segment::MIN))
                        } else {
                            e.get_prev(&E(Segment::MAX))
                        } {
                            cursor = TribleCursor::AE(e.clone(), *key, ascending);
                            valid = true;
                        }
                    } else if variable == self.variable_v1 {
                        if let Some((key, _)) = if ascending {
                            v1.get_next(&V1(Segment::MIN))
                        } else {
                            v1.get_prev(&V1(Segment::MAX))
                        } {
                            cursor = TribleCursor::AV1(v1.clone(), *key, ascending);
                            valid = true;
                        }
                    }
                }
                TribleCursor::V1(index, value, _) => {
                    let V1Branch { v2 } = index.get(value).unwrap();
                    if let Some((key, _)) = if ascending {
                        v2.get_next(&V2(Segment::MIN))
                    } else {
                        v2.get_prev(&V2(Segment::MAX))
                    } {
                        cursor = TribleCursor::V1V2(v2.clone(), *key, ascending);
                        valid = true;
                    }
                }

                TribleCursor::EA(index, value, _) => {
                    let EABranch { v1 } = index.get(value).unwrap();
                    if let Some((key, _)) = if ascending {
                        v1.get_next(&V1(Segment::MIN))
                    } else {
                        v1.get_prev(&V1(Segment::MAX))
                    } {
                        cursor = TribleCursor::EAV1(v1.clone(), *key, ascending);
                        valid = true;
                    }
                }
                TribleCursor::EAV1(index, value, _) => {
                    let EAV1Branch { v2 } = index.get(value).unwrap();
                    if let Some(key) = if ascending {
                        v2.get_next(&V2(Segment::MIN))
                    } else {
                        v2.get_prev(&V2(Segment::MAX))
                    } {
                        cursor = TribleCursor::EAV1V2(v2.clone(), *key, ascending);
                        valid = true;
                    }
                    done = true;
                }
                TribleCursor::EV1(index, value, _) => {
                    let EV1Branch { v2 } = index.get(value).unwrap();
                    if let Some((key, _)) = if ascending {
                        v2.get_next(&V2(Segment::MIN))
                    } else {
                        v2.get_prev(&V2(Segment::MAX))
                    } {
                        cursor = TribleCursor::EV1V2(v2.clone(), *key, ascending);
                        valid = true;
                    }
                }
                TribleCursor::EV1V2(index, value, _) => {
                    let EV1V2Branch { a } = index.get(value).unwrap();
                    if let Some(key) = if ascending {
                        a.get_next(&A(Segment::MIN))
                    } else {
                        a.get_prev(&A(Segment::MAX))
                    } {
                        cursor = TribleCursor::EV1V2A(a.clone(), *key, ascending);
                        valid = true;
                    }
                    done = true;
                }
                TribleCursor::AE(index, value, _) => {
                    let AEBranch { v1 } = index.get(value).unwrap();
                    if let Some((key, _)) = if ascending {
                        v1.get_next(&V1(Segment::MIN))
                    } else {
                        v1.get_prev(&V1(Segment::MAX))
                    } {
                        cursor = TribleCursor::AEV1(v1.clone(), *key, ascending);
                        valid = true;
                    }
                }
                TribleCursor::AEV1(index, value, _) => {
                    let AEV1Branch { v2 } = index.get(value).unwrap();
                    if let Some(key) = if ascending {
                        v2.get_next(&V2(Segment::MIN))
                    } else {
                        v2.get_prev(&V2(Segment::MAX))
                    } {
                        cursor = TribleCursor::AEV1V2(v2.clone(), *key, ascending);
                        valid = true;
                    }
                    done = true;
                }
                TribleCursor::AV1(index, value, _) => {
                    let AV1Branch { v2 } = index.get(value).unwrap();
                    if let Some((key, _)) = if ascending {
                        v2.get_next(&V2(Segment::MIN))
                    } else {
                        v2.get_prev(&V2(Segment::MAX))
                    } {
                        cursor = TribleCursor::AV1V2(v2.clone(), *key, ascending);
                        valid = true;
                    }
                }
                TribleCursor::AV1V2(index, value, _) => {
                    let AV1V2Branch { e } = index.get(value).unwrap();
                    if let Some(key) = if ascending {
                        e.get_next(&E(Segment::MIN))
                    } else {
                        e.get_prev(&E(Segment::MAX))
                    } {
                        cursor = TribleCursor::AV1V2E(e.clone(), *key, ascending);
                        valid = true;
                    }
                    done = true;
                }
                TribleCursor::V1V2(index, value, _) => {
                    if variable == self.variable_e {
                        let V1V2Branch { e, .. } = index.get(value).unwrap();
                        if let Some((key, _)) = if ascending {
                            e.get_next(&E(Segment::MIN))
                        } else {
                            e.get_prev(&E(Segment::MAX))
                        } {
                            cursor = TribleCursor::V1V2E(e.clone(), *key, ascending);
                            valid = true;
                        }
                    } else if variable == self.variable_a {
                        let V1V2Branch { a, .. } = index.get(value).unwrap();
                        if let Some((key, _)) = if ascending {
                            a.get_next(&A(Segment::MIN))
                        } else {
                            a.get_prev(&A(Segment::MAX))
                        } {
                            cursor = TribleCursor::V1V2A(a.clone(), *key, ascending);
                            valid = true;
                        }
                    }
                }
                TribleCursor::V1V2A(index, value, _) => {
                    let V1V2ABranch { e } = index.get(value).unwrap();
                    if let Some(key) = if ascending {
                        e.get_next(&E(Segment::MIN))
                    } else {
                        e.get_prev(&E(Segment::MAX))
                    } {
                        cursor = TribleCursor::V1V2AE(e.clone(), *key, ascending);
                        valid = true;
                    }
                    done = true;
                }
                TribleCursor::V1V2E(index, value, _) => {
                    let V1V2EBranch { a } = index.get(value).unwrap();
                    if let Some(key) = if ascending {
                        a.get_next(&A(Segment::MIN))
                    } else {
                        a.get_prev(&A(Segment::MAX))
                    } {
                        cursor = TribleCursor::V1V2EA(a.clone(), *key, ascending);
                        valid = true;
                    }
                    done = true;
                }
                TribleCursor::EAV1V2(..)
                | TribleCursor::EV1V2A(..)
                | TribleCursor::AEV1V2(..)
                | TribleCursor::AV1V2E(..)
                | TribleCursor::V1V2EA(..)
                | TribleCursor::V1V2AE(..)
                | TribleCursor::Invalid() => panic!("Should not have been reached for pushing!"),
            }

            self.cursors.push(cursor);
            self.valid = valid;

            return PushResult {
                relevant: true,
                done,
            };
        }

        fn pop(&mut self) {
            self.valid = true;
            self.cursors.pop();
        }
        fn valid(&self) -> bool {
            return self.valid;
        }
        fn peek(&self) -> Segment {
            match self.cursors.last().unwrap() {
                TribleCursor::E(_, value, _) => value.0,
                TribleCursor::A(_, value, _) => value.0,
                TribleCursor::V1(_, value, _) => value.0,
                TribleCursor::V1V2(_, value, _) => value.0,
                TribleCursor::EA(_, value, _) => value.0,
                TribleCursor::EAV1(_, value, _) => value.0,
                TribleCursor::EAV1V2(_, value, _) => value.0,
                TribleCursor::EV1(_, value, _) => value.0,
                TribleCursor::EV1V2(_, value, _) => value.0,
                TribleCursor::EV1V2A(_, value, _) => value.0,
                TribleCursor::AE(_, value, _) => value.0,
                TribleCursor::AEV1(_, value, _) => value.0,
                TribleCursor::AEV1V2(_, value, _) => value.0,
                TribleCursor::AV1(_, value, _) => value.0,
                TribleCursor::AV1V2(_, value, _) => value.0,
                TribleCursor::AV1V2E(_, value, _) => value.0,
                TribleCursor::V1V2E(_, value, _) => value.0,
                TribleCursor::V1V2EA(_, value, _) => value.0,
                TribleCursor::V1V2A(_, value, _) => value.0,
                TribleCursor::V1V2AE(_, value, _) => value.0,
                _ => panic!("Peeked invalid cursor!"),
            }
        }
        fn next(&mut self) {
            match self.cursors.last_mut().unwrap() {
                TribleCursor::E(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::A(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::V1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::V1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::EA(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::EAV1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::EAV1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::EV1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::EV1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::EV1V2A(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::AE(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::AEV1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::AEV1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::AV1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::AV1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::AV1V2E(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::V1V2E(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::V1V2EA(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::V1V2A(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                TribleCursor::V1V2AE(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return;
                    } else {
                        self.valid = false;
                        return;
                    }
                }
                _ => panic!("Peeked invalid cursor!"),
            }
        }
        fn seek(&mut self, value: Segment) -> bool {
            match self.cursors.last_mut().unwrap() {
                TribleCursor::E(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::A(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::V1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::V1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::EA(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::EAV1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::EAV1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::EV1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::EV1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::EV1V2A(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::AE(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::AEV1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::AEV1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::AV1(index, cursor, ascending) => {
                    if (*ascending && *cursor == V1(Segment::MAX))
                        || (!*ascending && *cursor == V1(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V1(cursor.0 + 1))
                    } else {
                        index.get_prev(&V1(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::AV1V2(index, cursor, ascending) => {
                    if (*ascending && *cursor == V2(Segment::MAX))
                        || (!*ascending && *cursor == V2(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&V2(cursor.0 + 1))
                    } else {
                        index.get_prev(&V2(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::AV1V2E(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::V1V2E(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::V1V2EA(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::V1V2A(index, cursor, ascending) => {
                    if (*ascending && *cursor == A(Segment::MAX))
                        || (!*ascending && *cursor == A(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some((key, _)) = if *ascending {
                        index.get_next(&A(cursor.0 + 1))
                    } else {
                        index.get_prev(&A(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                TribleCursor::V1V2AE(index, cursor, ascending) => {
                    if (*ascending && *cursor == E(Segment::MAX))
                        || (!*ascending && *cursor == E(Segment::MIN))
                    {
                        self.valid = false;
                        return false;
                    }
                    if let Some(key) = if *ascending {
                        index.get_next(&E(cursor.0 + 1))
                    } else {
                        index.get_prev(&E(cursor.0 - 1))
                    } {
                        *cursor = *key;
                        return cursor.0 == value;
                    } else {
                        self.valid = false;
                        return false;
                    }
                }
                _ => panic!("Peeked invalid cursor!"),
            }
        }
        */
}

/*


  propose() {

  }

  push(variable, ascending = true) {
    let branch;
    if (this.cursors.length === 0) {
      branch = this.db.index;
    } else {
      branch = this.cursors[this.cursors.length - 1].value();
    }

    const done = this.cursors.length === 3;
    if (variable === this.variableE) {
      this.cursors.push(branch.E.cursor(ascending));
      return { relevant: true, done };
    }
    if (variable === this.variableA) {
      this.cursors.push(branch.A.cursor(ascending));
      return { relevant: true, done };
    }
    if (variable === this.variableV1) {
      this.cursors.push(branch.V1.cursor(ascending));
      return { relevant: true, done };
    }
    if (variable === this.variableV2) {
      this.cursors.push(branch.V2.cursor(ascending));
      return { relevant: true, done };
    }
    return { relevant: false, done };
  }

  pop() {
    this.cursors.pop();
  }

  valid() {
    this.cursors[this.cursors.length - 1].valid;
  }

  peek() {
    if (this.cursors[this.cursors.length - 1].valid) {
      return this.cursor.peek();
    }
    return null;
  }

  next() {
    this.cursors[this.cursors.length - 1].next();
  }

  seek(value) {
    return this.cursors[this.cursors.length - 1].seek(value);
  }
}
*/
/*

  empty() {
    return new MemTribleDB();
  }

  isEmpty() {
    return this.indexE.isEmpty();
  }

  isEqual(other) {
    return this.indexE.isEqual(other.indexE);
  }

  isSubsetOf(other) {
    return this.indexE.isSubsetWith(
      other.indexE,
      ({ A: thisA }, { A: otherA }) =>
        thisA.isSubsetWith(
          otherA,
          ({ V: thisV }, { V: otherV }) => thisV.isSubset(otherV),
        ),
    );
  }

  isIntersecting(other) {
    return this.indexE.isIntersectingWith(
      other.indexE,
      ({ A: thisA }, { A: otherA }) =>
        thisA.isIntersectingWith(
          otherA,
          ({ V: thisV }, { V: otherV }) => thisV.isIntersecting(otherV),
        ),
    );
  }

  union(other) {
    const indexE = this.indexE.unionWith(
      other.indexE,
      (
        { A: thisA, V: thisV, AV: thisAV },
        { A: otherA, V: otherV, AV: otherAV },
      ) => ({
        A: thisA.unionWith(otherA, (thisV, otherV) => thisV.union(otherV)),
        V: thisV.unionWith(otherV, (thisA, otherA) => thisA.union(otherA)),
        AV: thisAV.union(otherAV),
      }),
    );
    const indexA = this.indexA.unionWith(
      other.indexA,
      (
        { E: thisE, V: thisV, EV: thisEV },
        { E: otherE, V: otherV, EV: otherEV },
      ) => ({
        E: thisE.unionWith(otherE, (thisV, otherV) => thisV.union(otherV)),
        V: thisV.unionWith(otherV, (thisE, otherE) => thisE.union(otherE)),
        EV: thisEV.union(otherEV),
      }),
    );
    const indexV = this.indexV.unionWith(
      other.indexV,
      (
        { E: thisE, A: thisA, EA: thisEA },
        { E: otherE, A: otherA, EA: otherEA },
      ) => ({
        E: thisE.unionWith(otherE, (thisA, otherA) => thisA.union(otherA)),
        A: thisA.unionWith(otherA, (thisE, otherE) => thisE.union(otherE)),
        EA: thisEA.union(otherEA),
      }),
    );
    const indexEA = this.indexEA.unionWith(
      other.indexEA,
      ({ V: thisV }, { V: otherV }) => thisV.union(otherV),
    );
    const indexEV = this.indexEV.unionWith(
      other.indexEV,
      ({ A: thisA }, { A: otherA }) => thisA.union(otherA),
    );
    const indexAV = this.indexAV.unionWith(
      other.indexAV,
      ({ E: thisE }, { E: otherE }) => thisE.union(otherE),
    );
    const indexEAV = this.indexEAV.union(other.indexEAV);
    return new MemTribleDB(
      indexE,
      indexA,
      indexV,
      indexEA,
      indexEV,
      indexAV,
      indexEAV,
    );
  }

  subtract(other) {
    const index = new Array(INDEX_COUNT);
    for (let i = 0; i < INDEX_COUNT; i++) {
      index[i] = this.index[i].subtract(other.index[i]);
    }
    return new MemTribleDB(index);
  }

  difference(other) {
    const index = new Array(INDEX_COUNT);
    for (let i = 0; i < INDEX_COUNT; i++) {
      index[i] = this.index[i].difference(other.index[i]);
    }
    return new MemTribleDB(index);
  }

  intersect(other) {
    const index = new Array(INDEX_COUNT);
    for (let i = 0; i < INDEX_COUNT; i++) {
      index[i] = this.index[i].intersect(other.index[i]);
    }
    return new MemTribleDB(index);
  }
}

export { MemTribleDB };
*/
