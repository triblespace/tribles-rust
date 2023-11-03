use crate::{
    query::{Binding, Constraint, Variable, VariableId, VariableSet},
    trible::Trible,
};

use super::*;

pub struct HashTribleSetConstraint<'a, E, A, V>
where
    E: From<Id>,
    A: From<Id>,
    V: From<Value>,
    for<'b> &'b E: Into<Id>,
    for<'b> &'b A: Into<Id>,
    for<'b> &'b V: Into<Value>,
{
    variable_e: Variable<E>,
    variable_a: Variable<A>,
    variable_v: Variable<V>,
    set: &'a HashTribleSet,
}

impl<'a, E, A, V> HashTribleSetConstraint<'a, E, A, V>
where
    E: From<Id>,
    A: From<Id>,
    V: From<Value>,
    for<'b> &'b E: Into<Id>,
    for<'b> &'b A: Into<Id>,
    for<'b> &'b V: Into<Value>,
{
    pub fn new(
        variable_e: Variable<E>,
        variable_a: Variable<A>,
        variable_v: Variable<V>,
        set: &'a HashTribleSet,
    ) -> Self {
        HashTribleSetConstraint {
            variable_e,
            variable_a,
            variable_v,
            set,
        }
    }
}

impl<'a, E, A, V> Constraint<'a> for HashTribleSetConstraint<'a, E, A, V>
where
    E: From<Id>,
    A: From<Id>,
    V: From<Value>,
    for<'b> &'b E: Into<Id>,
    for<'b> &'b A: Into<Id>,
    for<'b> &'b V: Into<Value>,
{
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e.index);
        variables.set(self.variable_a.index);
        variables.set(self.variable_v.index);
        variables
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable_e.index == variable ||
        self.variable_a.index == variable ||
        self.variable_v.index == variable
    }

    fn estimate(&self, variable: VariableId, binding: Binding) -> usize {
        let e_bound = binding.bound.is_set(self.variable_e.index);
        let a_bound = binding.bound.is_set(self.variable_a.index);
        let v_bound = binding.bound.is_set(self.variable_v.index);

        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        if let Some(trible) = Trible::new_values(
            binding.get(self.variable_e.index).unwrap_or([0; 32]),
            binding.get(self.variable_a.index).unwrap_or([0; 32]),
            binding.get(self.variable_v.index).unwrap_or([0; 32]),
        ) {
            match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
                (false, false, false, true, false, false) => self.set.ea.len(),
                (false, false, false, false, true, false) => self.set.ae.len(),
                (false, false, false, false, false, true) => self.set.ve.len(),

                (true, false, false, false, true, false) => {
                    self.set.ea.get(&trible.e()).map_or(0, |s| s.len())
                }
                (true, false, false, false, false, true) => {
                    self.set.ev.get(&trible.e()).map_or(0, |s| s.len())
                }

                (false, true, false, true, false, false) => {
                    self.set.ae.get(&trible.a()).map_or(0, |s| s.len())
                }
                (false, true, false, false, false, true) => {
                    self.set.av.get(&trible.a()).map_or(0, |s| s.len())
                }

                (false, false, true, true, false, false) => {
                    self.set.ve.get(&trible.v()).map_or(0, |s| s.len())
                }
                (false, false, true, false, true, false) => {
                    self.set.va.get(&trible.v()).map_or(0, |s| s.len())
                }

                (false, true, true, true, false, false) => self
                    .set
                    .ave
                    .get(&(trible.a(), trible.v()))
                    .map_or(0, |s| s.len()),
                (true, false, true, false, true, false) => self
                    .set
                    .eva
                    .get(&(trible.e(), trible.v()))
                    .map_or(0, |s| s.len()),
                (true, true, false, false, false, true) => self
                    .set
                    .eav
                    .get(&(trible.e(), trible.a()))
                    .map_or(0, |s| s.len()),
                _ => panic!(),
            }
        } else {
            0
        }
    }

    fn propose(&self, variable: VariableId, binding: Binding) -> Vec<Value> {
        let e_bound = binding.bound.is_set(self.variable_e.index);
        let a_bound = binding.bound.is_set(self.variable_a.index);
        let v_bound = binding.bound.is_set(self.variable_v.index);

        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        fn as_value(id: &Id) -> Value {
            let mut o: [u8; 32] = [0u8; 32];
            o[16..=31].copy_from_slice(id);
            o
        }

        if let Some(trible) = Trible::new_values(
            binding.get(self.variable_e.index).unwrap_or([0; 32]),
            binding.get(self.variable_a.index).unwrap_or([0; 32]),
            binding.get(self.variable_v.index).unwrap_or([0; 32]),
        ) {
            match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
                (false, false, false, true, false, false) => self
                    .set
                    .ea
                    .keys()
                    .map(|e| as_value(e))
                    .collect::<Vec<Value>>(),
                (false, false, false, false, true, false) => self
                    .set
                    .ae
                    .keys()
                    .map(|a| as_value(a))
                    .collect::<Vec<Value>>(),
                (false, false, false, false, false, true) => {
                    self.set.ve.keys().copied().collect::<Vec<Value>>()
                }

                (true, false, false, false, true, false) => {
                    self.set.ea.get(&trible.e()).map_or(vec![], |s| {
                        s.iter().map(|a| as_value(a)).collect::<Vec<Value>>()
                    })
                }
                (true, false, false, false, false, true) => self
                    .set
                    .ev
                    .get(&trible.e())
                    .map_or(vec![], |s| s.iter().copied().collect::<Vec<Value>>()),

                (false, true, false, true, false, false) => {
                    self.set.ae.get(&trible.a()).map_or(vec![], |s| {
                        s.iter().map(|e| as_value(e)).collect::<Vec<Value>>()
                    })
                }
                (false, true, false, false, false, true) => self
                    .set
                    .av
                    .get(&trible.a())
                    .map_or(vec![], |s| s.iter().copied().collect::<Vec<Value>>()),

                (false, false, true, true, false, false) => {
                    self.set.ve.get(&trible.v()).map_or(vec![], |s| {
                        s.iter().map(|e| as_value(e)).collect::<Vec<Value>>()
                    })
                }
                (false, false, true, false, true, false) => {
                    self.set.va.get(&trible.v()).map_or(vec![], |s| {
                        s.iter().map(|a| as_value(a)).collect::<Vec<Value>>()
                    })
                }

                (false, true, true, true, false, false) => self
                    .set
                    .ave
                    .get(&(trible.a(), trible.v()))
                    .map_or(vec![], |s| {
                        s.iter().map(|e| as_value(e)).collect::<Vec<Value>>()
                    }),
                (true, false, true, false, true, false) => self
                    .set
                    .eva
                    .get(&(trible.e(), trible.v()))
                    .map_or(vec![], |s| {
                        s.iter().map(|a| as_value(a)).collect::<Vec<Value>>()
                    }),
                (true, true, false, false, false, true) => self
                    .set
                    .eav
                    .get(&(trible.e(), trible.a()))
                    .map_or(vec![], |s| s.iter().copied().collect::<Vec<Value>>()),
                _ => panic!(),
            }
        } else {
            vec![]
        }
    }

    fn confirm(&self, variable: VariableId, binding: Binding, proposals: &mut Vec<Value>) {
        let e_bound = binding.bound.is_set(self.variable_e.index);
        let a_bound = binding.bound.is_set(self.variable_a.index);
        let v_bound = binding.bound.is_set(self.variable_v.index);

        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        match (e_bound || e_var, a_bound || a_var, v_bound || v_var) {
            (false, false, false) => panic!(),
            (true, false, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::new_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.ea.contains_key(&trible.e())
                    } else {
                        false
                    }
                });
            }
            (false, true, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::new_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.ae.contains_key(&trible.a())
                    } else {
                        false
                    }
                });
            }
            (false, false, true) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::new_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.ve.contains_key(&trible.v())
                    } else {
                        false
                    }
                });
            }

            (true, true, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::new_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.eav.contains_key(&(trible.e(), trible.a()))
                    } else {
                        false
                    }
                });
            }

            (true, false, true) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::new_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.eva.contains_key(&(trible.e(), trible.v()))
                    } else {
                        false
                    }
                });
            }

            (false, true, true) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::new_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.ave.contains_key(&(trible.a(), trible.v()))
                    } else {
                        false
                    }
                });
            }

            (true, true, true) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::new_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.all.contains(&trible)
                    } else {
                        false
                    }
                });
            }
        }
    }
}
