use core::panic;
//use std::convert::TryInto;
//use std::{collections::HashSet, fmt::Debug, hash::Hash};

use super::*;
use crate::query::*;
use crate::trible::*;

pub struct TribleSetConstraint<'a, E, A, V>
where
    E: Idlike,
    A: Idlike,
    V: Valuelike,
{
    variable_e: Variable<E>,
    variable_a: Variable<A>,
    variable_v: Variable<V>,
    set: &'a TribleSet,
}

impl<'a, E, A, V> TribleSetConstraint<'a, E, A, V>
where
    E: Idlike,
    A: Idlike,
    V: Valuelike,
{
    pub fn new(
        variable_e: Variable<E>,
        variable_a: Variable<A>,
        variable_v: Variable<V>,
        set: &'a TribleSet,
    ) -> Self {
        TribleSetConstraint {
            variable_e,
            variable_a,
            variable_v,
            set,
        }
    }
}

impl<'a, E, A, V> Constraint<'a> for TribleSetConstraint<'a, E, A, V>
where
    E: Idlike,
    A: Idlike,
    V: Valuelike,
{
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e.index);
        variables.set(self.variable_a.index);
        variables.set(self.variable_v.index);
        variables
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable_e.index == variable
            || self.variable_a.index == variable
            || self.variable_v.index == variable
    }

    fn estimate(&self, variable: VariableId, binding: Binding) -> usize {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index);
        let a_bound = binding.get(self.variable_a.index);
        let v_bound = binding.get(self.variable_v.index);

        (match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => {
                let trible = Trible::new_raw([0; 64]);
                self.set.eav.segmented_len(&trible.data, E_START)
            }
            (None, None, None, false, true, false) => {
                let trible = Trible::new_raw([0; 64]);
                self.set.aev.segmented_len(&trible.data, A_START)
            }
            (None, None, None, false, false, true) => {
                let trible = Trible::new_raw([0; 64]);
                self.set.vea.segmented_len(&trible.data, V_START)
            }

            (Some(e), None, None, false, true, false) => {
                let trible = Trible::new_raw_values(e, [0; 32], [0; 32]);
                self.set.eav.segmented_len(&trible.data, A_START)
            }
            (Some(e), None, None, false, false, true) => {
                let trible = Trible::new_raw_values(e, [0; 32], [0; 32]);
                self.set.eva.segmented_len(&trible.data, V_START)
            }

            (None, Some(a), None, true, false, false) => {
                let trible = Trible::new_raw_values([0; 32], a, [0; 32]);
                self.set.aev.segmented_len(&trible.data, E_START)
            }
            (None, Some(a), None, false, false, true) => {
                let trible = Trible::new_raw_values([0; 32], a, [0; 32]);
                self.set.ave.segmented_len(&trible.data, V_START)
            }

            (None, None, Some(v), true, false, false) => {
                let trible = Trible::new_raw_values([0; 32], [0; 32], v);
                self.set.vea.segmented_len(&trible.data, E_START)
            }
            (None, None, Some(v), false, true, false) => {
                let trible = Trible::new_raw_values([0; 32], [0; 32], v);
                self.set.vae.segmented_len(&trible.data, A_START)
            }

            (None, Some(a), Some(v), true, false, false) => {
                let trible = Trible::new_raw_values([0; 32], a, v);
                self.set.ave.segmented_len(&trible.data, E_START)
            }
            (Some(e), None, Some(v), false, true, false) => {
                let trible: Trible = Trible::new_raw_values(e, [0; 32], v);
                self.set.eva.segmented_len(&trible.data, A_START)
            }
            (Some(e), Some(a), None, false, false, true) => {
                let trible: Trible = Trible::new_raw_values(e, a, [0; 32]);
                self.set.eav.segmented_len(&trible.data, V_START)
            }
            _ => panic!(),
        }) as usize
    }

    fn propose(&self, variable: VariableId, binding: Binding) -> Vec<Value> {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index);
        let a_bound = binding.get(self.variable_a.index);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => {
                let trible = Trible::new_raw([0; 64]);
                self.set.eav.infixes(&trible.data, E_START, E_END, |k| {
                    Trible::new_raw(k).e_as_value()
                })
            }
            (None, None, None, false, true, false) => {
                let trible = Trible::new_raw([0; 64]);
                self.set.aev.infixes(&trible.data, A_START, A_END, |k| {
                    Trible::new_raw(k).a_as_value()
                })
            }
            (None, None, None, false, false, true) => {
                let trible = Trible::new_raw([0; 64]);
                self.set
                    .vea
                    .infixes(&trible.data, V_START, V_END, |k| Trible::new_raw(k).v())
            }

            (Some(e), None, None, false, true, false) => {
                let trible = Trible::new_raw_values(e, [0; 32], [0; 32]);
                self.set.eav.infixes(&trible.data, A_START, A_END, |k| {
                    Trible::new_raw(k).a_as_value()
                })
            }
            (Some(e), None, None, false, false, true) => {
                let trible = Trible::new_raw_values(e, [0; 32], [0; 32]);
                self.set
                    .eva
                    .infixes(&trible.data, V_START, V_END, |k| Trible::new_raw(k).v())
            }

            (None, Some(a), None, true, false, false) => {
                let trible = Trible::new_raw_values([0; 32], a, [0; 32]);
                self.set.aev.infixes(&trible.data, E_START, E_END, |k| {
                    Trible::new_raw(k).e_as_value()
                })
            }
            (None, Some(a), None, false, false, true) => {
                let trible = Trible::new_raw_values([0; 32], a, [0; 32]);
                self.set
                    .ave
                    .infixes(&trible.data, V_START, V_END, |k| Trible::new_raw(k).v())
            }

            (None, None, Some(v), true, false, false) => {
                let trible = Trible::new_raw_values([0; 32], [0; 32], v);
                self.set.vea.infixes(&trible.data, E_START, E_END, |k| {
                    Trible::new_raw(k).e_as_value()
                })
            }
            (None, None, Some(v), false, true, false) => {
                let trible = Trible::new_raw_values([0; 32], [0; 32], v);
                self.set.vae.infixes(&trible.data, A_START, A_END, |k| {
                    Trible::new_raw(k).a_as_value()
                })
            }
            (None, Some(a), Some(v), true, false, false) => {
                let trible = Trible::new_raw_values([0; 32], a, v);
                self.set.ave.infixes(&trible.data, E_START, E_END, |k| {
                    Trible::new_raw(k).e_as_value()
                })
            }
            (Some(e), None, Some(v), false, true, false) => {
                let trible = Trible::new_raw_values(e, [0; 32], v);
                self.set.eva.infixes(&trible.data, A_START, A_END, |k| {
                    Trible::new_raw(k).a_as_value()
                })
            }
            (Some(e), Some(a), None, false, false, true) => {
                let trible = Trible::new_raw_values(e, a, [0; 32]);
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
