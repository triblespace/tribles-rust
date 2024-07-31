use crate::{
    query::{Binding, Constraint, Variable, VariableId, VariableSet},
    trible::Trible,
};

use super::*;

pub struct HashTribleSetConstraint<'a>
{
    variable_e: VariableId,
    variable_a: VariableId,
    variable_v: VariableId,
    set: &'a HashTribleSet,
}

impl<'a> HashTribleSetConstraint<'a>
{
    pub fn new<V>(
        variable_e: Variable<Id>,
        variable_a: Variable<Id>,
        variable_v: Variable<V>,
        set: &'a HashTribleSet,
    ) -> Self {
        HashTribleSetConstraint {
            variable_e: variable_e.index,
            variable_a: variable_a.index,
            variable_v: variable_v.index,
            set,
        }
    }
}

impl<'a> Constraint<'a> for HashTribleSetConstraint<'a>
{
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e);
        variables.set(self.variable_a);
        variables.set(self.variable_v);
        variables
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable_e == variable
            || self.variable_a == variable
            || self.variable_v == variable
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        let e_bound = binding.bound.is_set(self.variable_e);
        let a_bound = binding.bound.is_set(self.variable_a);
        let v_bound = binding.bound.is_set(self.variable_v);

        let e_var = self.variable_e == variable;
        let a_var = self.variable_a == variable;
        let v_var = self.variable_v == variable;

        if let Ok(trible) = Trible::new_values(
            binding.get(self.variable_e).unwrap_or([0; 32]),
            binding.get(self.variable_a).unwrap_or([0; 32]),
            binding.get(self.variable_v).unwrap_or([0; 32]),
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

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<RawValue> {
        let e_bound = binding.bound.is_set(self.variable_e);
        let a_bound = binding.bound.is_set(self.variable_a);
        let v_bound = binding.bound.is_set(self.variable_v);

        let e_var = self.variable_e == variable;
        let a_var = self.variable_a == variable;
        let v_var = self.variable_v == variable;

        fn as_value(id: &RawId) -> RawValue {
            let mut o: [u8; 32] = [0u8; 32];
            o[16..=31].copy_from_slice(id);
            o
        }

        if let Ok(trible) = Trible::new_values(
            binding.get(self.variable_e).unwrap_or([0; 32]),
            binding.get(self.variable_a).unwrap_or([0; 32]),
            binding.get(self.variable_v).unwrap_or([0; 32]),
        ) {
            match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
                (false, false, false, true, false, false) => self
                    .set
                    .ea
                    .keys()
                    .map(|e| as_value(e))
                    .collect::<Vec<RawValue>>(),
                (false, false, false, false, true, false) => self
                    .set
                    .ae
                    .keys()
                    .map(|a| as_value(a))
                    .collect::<Vec<RawValue>>(),
                (false, false, false, false, false, true) => {
                    self.set.ve.keys().copied().collect::<Vec<RawValue>>()
                }

                (true, false, false, false, true, false) => {
                    self.set.ea.get(&trible.e()).map_or(vec![], |s| {
                        s.iter().map(|a| as_value(a)).collect::<Vec<RawValue>>()
                    })
                }
                (true, false, false, false, false, true) => self
                    .set
                    .ev
                    .get(&trible.e())
                    .map_or(vec![], |s| s.iter().copied().collect::<Vec<RawValue>>()),

                (false, true, false, true, false, false) => {
                    self.set.ae.get(&trible.a()).map_or(vec![], |s| {
                        s.iter().map(|e| as_value(e)).collect::<Vec<RawValue>>()
                    })
                }
                (false, true, false, false, false, true) => self
                    .set
                    .av
                    .get(&trible.a())
                    .map_or(vec![], |s| s.iter().copied().collect::<Vec<RawValue>>()),

                (false, false, true, true, false, false) => {
                    self.set.ve.get(&trible.v()).map_or(vec![], |s| {
                        s.iter().map(|e| as_value(e)).collect::<Vec<RawValue>>()
                    })
                }
                (false, false, true, false, true, false) => {
                    self.set.va.get(&trible.v()).map_or(vec![], |s| {
                        s.iter().map(|a| as_value(a)).collect::<Vec<RawValue>>()
                    })
                }

                (false, true, true, true, false, false) => self
                    .set
                    .ave
                    .get(&(trible.a(), trible.v()))
                    .map_or(vec![], |s| {
                        s.iter().map(|e| as_value(e)).collect::<Vec<RawValue>>()
                    }),
                (true, false, true, false, true, false) => self
                    .set
                    .eva
                    .get(&(trible.e(), trible.v()))
                    .map_or(vec![], |s| {
                        s.iter().map(|a| as_value(a)).collect::<Vec<RawValue>>()
                    }),
                (true, true, false, false, false, true) => self
                    .set
                    .eav
                    .get(&(trible.e(), trible.a()))
                    .map_or(vec![], |s| s.iter().copied().collect::<Vec<RawValue>>()),
                _ => panic!(),
            }
        } else {
            vec![]
        }
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let e_bound = binding.bound.is_set(self.variable_e);
        let a_bound = binding.bound.is_set(self.variable_a);
        let v_bound = binding.bound.is_set(self.variable_v);

        let e_var = self.variable_e == variable;
        let a_var = self.variable_a == variable;
        let v_var = self.variable_v == variable;

        match (e_bound || e_var, a_bound || a_var, v_bound || v_var) {
            (false, false, false) => panic!(),
            (true, false, false) => {
                proposals.retain(|value| {
                    if let Ok(trible) = Trible::new_values(
                        binding.get(self.variable_e).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v).unwrap_or(if v_var {
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
                    if let Ok(trible) = Trible::new_values(
                        binding.get(self.variable_e).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v).unwrap_or(if v_var {
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
                    if let Ok(trible) = Trible::new_values(
                        binding.get(self.variable_e).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v).unwrap_or(if v_var {
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
                    if let Ok(trible) = Trible::new_values(
                        binding.get(self.variable_e).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v).unwrap_or(if v_var {
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
                    if let Ok(trible) = Trible::new_values(
                        binding.get(self.variable_e).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v).unwrap_or(if v_var {
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
                    if let Ok(trible) = Trible::new_values(
                        binding.get(self.variable_e).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v).unwrap_or(if v_var {
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
                    if let Ok(trible) = Trible::new_values(
                        binding.get(self.variable_e).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v).unwrap_or(if v_var {
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
