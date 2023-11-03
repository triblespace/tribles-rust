use core::panic;
//use std::convert::TryInto;
//use std::{collections::HashSet, fmt::Debug, hash::Hash};

use super::*;
use crate::trible::*;
use crate::query::*;

pub struct PATCHTribleSetConstraint<'a, E, A, V>
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
    set: &'a PATCHTribleSet,
}

impl<'a, E, A, V> PATCHTribleSetConstraint<'a, E, A, V>
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
        set: &'a PATCHTribleSet,
    ) -> Self {
        PATCHTribleSetConstraint {
            variable_e,
            variable_a,
            variable_v,
            set,
        }
    }
}

impl<'a, E, A, V> Constraint<'a> for PATCHTribleSetConstraint<'a, E, A, V>
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

        let trible = Trible::new_raw_values(
            binding.get(self.variable_e.index).unwrap_or([0; 32]),
            binding.get(self.variable_a.index).unwrap_or([0; 32]),
            binding.get(self.variable_v.index).unwrap_or([0; 32]),
        );

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (false, false, false, true, false, false) => {
                self.set.eav.segmented_len(&trible.data, E_START)
            }
            (false, false, false, false, true, false) => {
                self.set.aev.segmented_len(&trible.data, A_START)
            }
            (false, false, false, false, false, true) => {
                self.set.vea.segmented_len(&trible.data, V_START)
            }

            (true, false, false, false, true, false) => {
                self.set.eav.segmented_len(&trible.data, A_START)
            }
            (true, false, false, false, false, true) => {
                self.set.eva.segmented_len(&trible.data, V_START)
            }

            (false, true, false, true, false, false) => {
                self.set.aev.segmented_len(&trible.data, E_START)
            }
            (false, true, false, false, false, true) => {
                self.set.ave.segmented_len(&trible.data, V_START)
            }

            (false, false, true, true, false, false) => {
                self.set.vea.segmented_len(&trible.data, E_START)
            }
            (false, false, true, false, true, false) => {
                self.set.vae.segmented_len(&trible.data, A_START)
            }

            (false, true, true, true, false, false) => {
                self.set.ave.segmented_len(&trible.data, E_START)
            }
            (true, false, true, false, true, false) => {
                self.set.eva.segmented_len(&trible.data, A_START)
            }
            (true, true, false, false, false, true) => {
                self.set.eav.segmented_len(&trible.data, V_START)
            }
            _ => panic!(),
        }
    }

    fn propose(&self, variable: VariableId, binding: Binding) -> Vec<Value> {
        let e_bound = binding.bound.is_set(self.variable_e.index);
        let a_bound = binding.bound.is_set(self.variable_a.index);
        let v_bound = binding.bound.is_set(self.variable_v.index);

        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let trible =Trible::new_raw_values(
            binding.get(self.variable_e.index).unwrap_or([0; 32]),
            binding.get(self.variable_a.index).unwrap_or([0; 32]),
            binding.get(self.variable_v.index).unwrap_or([0; 32]),
        );

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (false, false, false, true, false, false) => {
                self.set.eav.infixes(&trible.data, E_START, E_END, |k| {
                    Trible::new_raw(k).e_as_value()
                })
            }
            (false, false, false, false, true, false) => {
                self.set.aev.infixes(&trible.data, A_START, A_END, |k| {
                    Trible::new_raw(k).a_as_value()
                })
            }
            (false, false, false, false, false, true) => {
                self.set
                    .vea
                    .infixes(&trible.data, V_START, V_END, |k| Trible::new_raw(k).v())
            }

            (true, false, false, false, true, false) => {
                self.set.eav.infixes(&trible.data, A_START, A_END, |k| {
                    Trible::new_raw(k).a_as_value()
                })
            }
            (true, false, false, false, false, true) => {
                self.set
                    .eva
                    .infixes(&trible.data, V_START, V_END, |k| Trible::new_raw(k).v())
            }

            (false, true, false, true, false, false) => {
                self.set.aev.infixes(&trible.data, E_START, E_END, |k| {
                    Trible::new_raw(k).e_as_value()
                })
            }
            (false, true, false, false, false, true) => {
                self.set
                    .ave
                    .infixes(&trible.data, V_START, V_END, |k| Trible::new_raw(k).v())
            }

            (false, false, true, true, false, false) => {
                self.set.vea.infixes(&trible.data, E_START, E_END, |k| {
                    Trible::new_raw(k).e_as_value()
                })
            }
            (false, false, true, false, true, false) => {
                self.set.vae.infixes(&trible.data, A_START, A_END, |k| {
                    Trible::new_raw(k).a_as_value()
                })
            }
            (false, true, true, true, false, false) => {
                self.set.ave.infixes(&trible.data, E_START, E_END, |k| {
                    Trible::new_raw(k).e_as_value()
                })
            }
            (true, false, true, false, true, false) => {
                self.set.eva.infixes(&trible.data, A_START, A_END, |k| {
                    Trible::new_raw(k).a_as_value()
                })
            }
            (true, true, false, false, false, true) => {
                self.set
                    .eav
                    .infixes(&trible.data, V_START, V_END, |k| Trible::new_raw(k).v())
            }
            _ => panic!(),
        }
    }

    fn confirm(&self, variable: VariableId, binding: Binding, proposals: &mut Vec<Value>) {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index);
        let a_bound = binding.get(self.variable_a.index);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => proposals.retain(|value| {
                let trible = Trible::new_raw_values(*value, [0; 32], [0; 32]);
                self.set.eav.has_prefix(&trible.data, E_END)
            }),
            (None, None, None, false, true, false) => proposals.retain(|value| {
                let trible = Trible::new_raw_values([0; 32], *value, [0; 32]);
                self.set.aev.has_prefix(&trible.data, A_END)
            }),
            (None, None, None, false, false, true) => proposals.retain(|value| {
                let trible = Trible::new_raw_values([0; 32], [0; 32], *value);
                self.set.vea.has_prefix(&trible.data, V_END)
            }),
            (Some(e), None, None, false, true, false) => proposals.retain(|value| {
                let trible = Trible::new_raw_values(e, *value, [0; 32]);
                self.set.eav.has_prefix(&trible.data, A_END)
            }),
            (Some(e), None, None, false, false, true) => proposals.retain(|value| {
                let trible = Trible::new_raw_values(e, [0; 32], *value);
                self.set.eva.has_prefix(&trible.data, V_END)
            }),
            (None, Some(a), None, true, false, false) => proposals.retain(|value| {
                let trible = Trible::new_raw_values(*value, a, [0; 32]);
                self.set.aev.has_prefix(&trible.data, E_END)
            }),
            (None, Some(a), None, false, false, true) => proposals.retain(|value| {
                let trible = Trible::new_raw_values([0; 32], a, *value);
                self.set.ave.has_prefix(&trible.data, V_END)
            }),
            (None, None, Some(v), true, false, false) => proposals.retain(|value| {
                let trible = Trible::new_raw_values(*value, [0; 32], v);
                self.set.vea.has_prefix(&trible.data, E_END)
            }),
            (None, None, Some(v), false, true, false) => proposals.retain(|value| {
                let trible = Trible::new_raw_values([0; 32], *value, v);
                self.set.vae.has_prefix(&trible.data, A_END)
            }),
            (None, Some(a), Some(v), true, false, false) => proposals.retain(|value: &[u8; 32]| {
                let trible = Trible::new_raw_values(*value, a, v);
                self.set.ave.has_prefix(&trible.data, E_END)
            }),
            (Some(e), None, Some(v), false, true, false) => proposals.retain(|value: &[u8; 32]| {
                let trible = Trible::new_raw_values(e, *value, v);
                self.set.eva.has_prefix(&trible.data, A_END)
            }),
            (Some(e), Some(a), None, false, false, true) => proposals.retain(|value: &[u8; 32]| {
                let trible = Trible::new_raw_values(e, a, *value);
                self.set.eav.has_prefix(&trible.data, V_END)
            }),
            _ => panic!("invalid trible constraint state"),
        }
    }
}
