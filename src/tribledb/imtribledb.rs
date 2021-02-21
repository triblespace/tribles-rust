use crate::trible::*;
use crate::tribledb::query::*;
use crate::tribledb::TribleDB;
use im::OrdMap;
use im::OrdSet;

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

#[derive(Clone)]
struct IndexBranch {
    e: OrdMap<E, EBranch>,
    a: OrdMap<A, ABranch>,
    v1: OrdMap<V1, V1Branch>,
    ea: OrdSet<Segment>,
    ev1: OrdSet<Segment>,
    av1: OrdSet<Segment>,
    eav1: OrdSet<Segment>,
}

#[derive(Clone)]
struct EBranch {
    a: OrdMap<A, EABranch>,
    v1: OrdMap<V1, EV1Branch>,
}

#[derive(Clone)]
struct EABranch {
    v1: OrdMap<V1, EAV1Branch>,
}

#[derive(Clone)]
struct EAV1Branch {
    v2: OrdSet<V2>,
}

#[derive(Clone)]
struct EV1Branch {
    v2: OrdMap<V2, EV1V2Branch>,
}

#[derive(Clone)]
struct EV1V2Branch {
    a: OrdSet<A>,
}

#[derive(Clone)]
struct ABranch {
    e: OrdMap<E, AEBranch>,
    v1: OrdMap<V1, AV1Branch>,
}

#[derive(Clone)]
struct AEBranch {
    v1: OrdMap<V1, AEV1Branch>,
}

#[derive(Clone)]
struct AEV1Branch {
    v2: OrdSet<V2>,
}

#[derive(Clone)]
struct AV1Branch {
    v2: OrdMap<V2, AV1V2Branch>,
}

#[derive(Clone)]
struct AV1V2Branch {
    e: OrdSet<E>,
}

#[derive(Clone)]
struct V1Branch {
    v2: OrdMap<V2, V1V2Branch>,
}

#[derive(Clone)]
struct V1V2Branch {
    e: OrdMap<E, V1V2EBranch>,
    a: OrdMap<A, V1V2ABranch>,
}

#[derive(Clone)]
struct V1V2ABranch {
    e: OrdSet<E>,
}

#[derive(Clone)]
struct V1V2EBranch {
    a: OrdSet<A>,
}

#[derive(Clone)]
pub struct ImTribleDB {
    index: IndexBranch,
}

impl Default for ImTribleDB {
    fn default() -> Self {
        ImTribleDB {
            index: IndexBranch {
                e: OrdMap::new(),
                a: OrdMap::new(),
                v1: OrdMap::new(),
                ea: OrdSet::new(),
                ev1: OrdSet::new(),
                av1: OrdSet::new(),
                eav1: OrdSet::new(),
            },
        }
    }
}

impl TribleDB for ImTribleDB {
    fn with<T>(&self, tribles: T) -> ImTribleDB
    where
        T: IntoIterator<Item = Trible>,
    {
        let mut index = self.index.clone();
        for trible in tribles {
            index.e = index.e.alter(
                |branch| match branch {
                    Some(EBranch { a, v1 }) => Some(EBranch {
                        a: a.alter(
                            |branch| match branch {
                                Some(EABranch { v1 }) => Some(EABranch {
                                    v1: v1.alter(
                                        |branch| match branch {
                                            Some(EAV1Branch { v2 }) => Some(EAV1Branch {
                                                v2: v2.update(trible.v2),
                                            }),
                                            None => Some(EAV1Branch {
                                                v2: OrdSet::unit(trible.v2),
                                            }),
                                        },
                                        trible.v1,
                                    ),
                                }),
                                None => Some(EABranch {
                                    v1: OrdMap::unit(
                                        trible.v1,
                                        EAV1Branch {
                                            v2: OrdSet::unit(trible.v2),
                                        },
                                    ),
                                }),
                            },
                            trible.a,
                        ),
                        v1: v1.alter(
                            |branch| match branch {
                                Some(EV1Branch { v2 }) => Some(EV1Branch {
                                    v2: v2.alter(
                                        |branch| match branch {
                                            Some(EV1V2Branch { a }) => Some(EV1V2Branch {
                                                a: a.update(trible.a),
                                            }),
                                            None => Some(EV1V2Branch {
                                                a: OrdSet::unit(trible.a),
                                            }),
                                        },
                                        trible.v2,
                                    ),
                                }),
                                None => Some(EV1Branch {
                                    v2: OrdMap::unit(
                                        trible.v2,
                                        EV1V2Branch {
                                            a: OrdSet::unit(trible.a),
                                        },
                                    ),
                                }),
                            },
                            trible.v1,
                        ),
                    }),
                    None => Some(EBranch {
                        a: OrdMap::unit(
                            trible.a,
                            EABranch {
                                v1: OrdMap::unit(
                                    trible.v1,
                                    EAV1Branch {
                                        v2: OrdSet::unit(trible.v2),
                                    },
                                ),
                            },
                        ),
                        v1: OrdMap::unit(
                            trible.v1,
                            EV1Branch {
                                v2: OrdMap::unit(
                                    trible.v2,
                                    EV1V2Branch {
                                        a: OrdSet::unit(trible.a),
                                    },
                                ),
                            },
                        ),
                    }),
                },
                trible.e,
            );
            index.a = index.a.alter(
                |branch| match branch {
                    Some(ABranch { e, v1 }) => Some(ABranch {
                        e: e.alter(
                            |branch| match branch {
                                Some(AEBranch { v1 }) => Some(AEBranch {
                                    v1: v1.alter(
                                        |branch| match branch {
                                            Some(AEV1Branch { v2 }) => Some(AEV1Branch {
                                                v2: v2.update(trible.v2),
                                            }),
                                            None => Some(AEV1Branch {
                                                v2: OrdSet::unit(trible.v2),
                                            }),
                                        },
                                        trible.v1,
                                    ),
                                }),
                                None => Some(AEBranch {
                                    v1: OrdMap::unit(
                                        trible.v1,
                                        AEV1Branch {
                                            v2: OrdSet::unit(trible.v2),
                                        },
                                    ),
                                }),
                            },
                            trible.e,
                        ),
                        v1: v1.alter(
                            |branch| match branch {
                                Some(AV1Branch { v2 }) => Some(AV1Branch {
                                    v2: v2.alter(
                                        |branch| match branch {
                                            Some(AV1V2Branch { e }) => Some(AV1V2Branch {
                                                e: e.update(trible.e),
                                            }),
                                            None => Some(AV1V2Branch {
                                                e: OrdSet::unit(trible.e),
                                            }),
                                        },
                                        trible.v2,
                                    ),
                                }),
                                None => Some(AV1Branch {
                                    v2: OrdMap::unit(
                                        trible.v2,
                                        AV1V2Branch {
                                            e: OrdSet::unit(trible.e),
                                        },
                                    ),
                                }),
                            },
                            trible.v1,
                        ),
                    }),
                    None => Some(ABranch {
                        e: OrdMap::unit(
                            trible.e,
                            AEBranch {
                                v1: OrdMap::unit(
                                    trible.v1,
                                    AEV1Branch {
                                        v2: OrdSet::unit(trible.v2),
                                    },
                                ),
                            },
                        ),
                        v1: OrdMap::unit(
                            trible.v1,
                            AV1Branch {
                                v2: OrdMap::unit(
                                    trible.v2,
                                    AV1V2Branch {
                                        e: OrdSet::unit(trible.e),
                                    },
                                ),
                            },
                        ),
                    }),
                },
                trible.a,
            );

            index.v1 = index.v1.alter(
                |branch| match branch {
                    Some(V1Branch { v2 }) => Some(V1Branch {
                        v2: v2.alter(
                            |branch| match branch {
                                Some(V1V2Branch { e, a }) => Some(V1V2Branch {
                                    e: e.alter(
                                        |branch| match branch {
                                            Some(V1V2EBranch { a }) => Some(V1V2EBranch {
                                                a: a.update(trible.a),
                                            }),
                                            None => Some(V1V2EBranch {
                                                a: OrdSet::unit(trible.a),
                                            }),
                                        },
                                        trible.e,
                                    ),
                                    a: a.alter(
                                        |branch| match branch {
                                            Some(V1V2ABranch { e }) => Some(V1V2ABranch {
                                                e: e.update(trible.e),
                                            }),
                                            None => Some(V1V2ABranch {
                                                e: OrdSet::unit(trible.e),
                                            }),
                                        },
                                        trible.a,
                                    ),
                                }),
                                None => Some(V1V2Branch {
                                    e: OrdMap::unit(
                                        trible.e,
                                        V1V2EBranch {
                                            a: OrdSet::unit(trible.a),
                                        },
                                    ),
                                    a: OrdMap::unit(
                                        trible.a,
                                        V1V2ABranch {
                                            e: OrdSet::unit(trible.e),
                                        },
                                    ),
                                }),
                            },
                            trible.v2,
                        ),
                    }),
                    None => Some(V1Branch {
                        v2: OrdMap::unit(
                            trible.v2,
                            V1V2Branch {
                                e: OrdMap::unit(
                                    trible.e,
                                    V1V2EBranch {
                                        a: OrdSet::unit(trible.a),
                                    },
                                ),
                                a: OrdMap::unit(
                                    trible.a,
                                    V1V2ABranch {
                                        e: OrdSet::unit(trible.e),
                                    },
                                ),
                            },
                        ),
                    }),
                },
                trible.v1,
            );

            if trible.e.0 == trible.a.0 {
                index.ea = index.ea.update(trible.e.0);
            }

            if trible.a.0 == trible.v1.0 {
                index.av1 = index.av1.update(trible.a.0);
            }

            if trible.e.0 == trible.v1.0 {
                index.ev1 = index.ev1.update(trible.e.0);
            }

            if trible.e.0 == trible.a.0 && trible.a.0 == trible.v1.0 {
                index.eav1 = index.eav1.update(trible.e.0);
            }
        }

        return ImTribleDB { index };
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
    */
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
