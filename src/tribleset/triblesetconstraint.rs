use core::panic;
//use std::convert::TryInto;
//use std::{collections::HashSet, fmt::Debug, hash::Hash};

use im::Vector;

use super::*;
use crate::id_from_value;
use crate::id_into_value;
use crate::query::*;
use crate::Id;
use crate::ID_LEN;
use crate::VALUE_LEN;

pub struct TribleSetConstraint<'a, V>
where
    V: Valuelike,
{
    variable_e: Variable<Id>,
    variable_a: Variable<Id>,
    variable_v: Variable<V>,
    set: &'a TribleSet,
}

impl<'a, V> TribleSetConstraint<'a, V>
where
    V: Valuelike,
{
    pub fn new(
        variable_e: Variable<Id>,
        variable_a: Variable<Id>,
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

impl<'a, V> Constraint<'a> for TribleSetConstraint<'a, V>
where
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

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index).map(id_from_value);
        let a_bound = binding.get(self.variable_a.index).map(id_from_value);
        let v_bound = binding.get(self.variable_v.index);

        (match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => self.set.eav.segmented_len(&[0; 0]),
            (None, None, None, false, true, false) => self.set.aev.segmented_len(&[0; 0]),
            (None, None, None, false, false, true) => self.set.vea.segmented_len(&[0; 0]),
            (Some(e), None, None, false, true, false) => {
                let mut prefix = [0u8; ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                self.set.eav.segmented_len(&prefix)
            }
            (Some(e), None, None, false, false, true) => {
                let mut prefix = [0u8; ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                self.set.eva.segmented_len(&prefix)
            }
            (None, Some(a), None, true, false, false) => {
                let mut prefix = [0u8; ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                self.set.aev.segmented_len(&prefix)
            }
            (None, Some(a), None, false, false, true) => {
                let mut prefix = [0u8; ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                self.set.ave.segmented_len(&prefix)
            }
            (None, None, Some(v), true, false, false) => {
                let mut prefix = [0u8; VALUE_LEN];
                prefix[0..VALUE_LEN].copy_from_slice(&v[..]);
                self.set.vea.segmented_len(&prefix)
            }
            (None, None, Some(v), false, true, false) => {
                let mut prefix = [0u8; VALUE_LEN];
                prefix[0..VALUE_LEN].copy_from_slice(&v[..]);
                self.set.vae.segmented_len(&prefix)
            }
            (None, Some(a), Some(v), true, false, false) => {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v);
                self.set.ave.segmented_len(&prefix)
            }
            (Some(e), None, Some(v), false, true, false) => {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v);
                self.set.eva.segmented_len(&prefix)
            }
            (Some(e), Some(a), None, false, false, true) => {
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&a);
                self.set.eav.segmented_len(&prefix)
            }
            _ => panic!(),
        }) as usize
    }

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<Value> {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index).map(id_from_value);
        let a_bound = binding.get(self.variable_a.index).map(id_from_value);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => {
                let estimate = self.set.eav.segmented_len(&[0; 0]) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set
                    .eav
                    .infixes(&[0; 0], &mut |e| r.push(id_into_value(e)));
                r
            }
            (None, None, None, false, true, false) => {
                let estimate = self.set.aev.segmented_len(&[0; 0]) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set
                    .aev
                    .infixes(&[0; 0], &mut |a| r.push(id_into_value(a)));
                r
            }
            (None, None, None, false, false, true) => {
                let estimate = self.set.vea.segmented_len(&[0; 0]) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set.vea.infixes(&[0; 0], &mut |v| r.push(v));
                r
            }

            (Some(e), None, None, false, true, false) => {
                let estimate = self.set.eav.segmented_len(&e) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set.eav.infixes(&e, &mut |a| r.push(id_into_value(a)));
                r
            }
            (Some(e), None, None, false, false, true) => {
                let estimate = self.set.eva.segmented_len(&e) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set.eva.infixes(&e, &mut |v| r.push(v));
                r
            }

            (None, Some(a), None, true, false, false) => {
                let estimate = self.set.aev.segmented_len(&a) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set.aev.infixes(&a, &mut |e| r.push(id_into_value(e)));
                r
            }
            (None, Some(a), None, false, false, true) => {
                let estimate = self.set.ave.segmented_len(&a) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set.ave.infixes(&a, &mut |v| r.push(v));
                r
            }

            (None, None, Some(v), true, false, false) => {
                let estimate = self.set.vea.segmented_len(&v) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set.vea.infixes(&v, &mut |e| r.push(id_into_value(e)));
                r
            }
            (None, None, Some(v), false, true, false) => {
                let estimate = self.set.vae.segmented_len(&v) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set.vae.infixes(&v, &mut |a| r.push(id_into_value(a)));
                r
            }
            (None, Some(a), Some(v), true, false, false) => {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v[..]);
                let estimate = self.set.ave.segmented_len(&prefix) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set
                    .ave
                    .infixes(&prefix, &mut |e| r.push(id_into_value(e)));
                r
            }
            (Some(e), None, Some(v), false, true, false) => {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v[..]);
                let estimate = self.set.eva.segmented_len(&prefix) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set
                    .eva
                    .infixes(&prefix, &mut |a| r.push(id_into_value(a)));
                r
            }
            (Some(e), Some(a), None, false, false, true) => {
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&a[..]);
                let estimate = self.set.eav.segmented_len(&prefix) as usize;
                let mut r = Vec::with_capacity(estimate);
                self.set.eav.infixes(&prefix, &mut |v| r.push(v));
                r
            }
            _ => panic!(),
        }
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<Value>) {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index).map(id_from_value);
        let a_bound = binding.get(self.variable_a.index).map(id_from_value);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => {
                proposals.retain(|value| self.set.eav.has_prefix(&id_from_value(*value)))
            }
            (None, None, None, false, true, false) => {
                proposals.retain(|value| self.set.aev.has_prefix(&id_from_value(*value)))
            }
            (None, None, None, false, false, true) => {
                proposals.retain(|value| self.set.vea.has_prefix(value))
            }
            (Some(e), None, None, false, true, false) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&id_from_value(*value));
                self.set.eav.has_prefix(&prefix)
            }),
            (Some(e), None, None, false, false, true) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(value);
                self.set.eva.has_prefix(&prefix)
            }),
            (None, Some(a), None, true, false, false) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&id_from_value(*value));
                self.set.aev.has_prefix(&prefix)
            }),
            (None, Some(a), None, false, false, true) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(value);
                self.set.ave.has_prefix(&prefix)
            }),
            (None, None, Some(v), true, false, false) => proposals.retain(|value| {
                let mut prefix = [0u8; VALUE_LEN + ID_LEN];
                prefix[0..VALUE_LEN].copy_from_slice(&v[..]);
                prefix[VALUE_LEN..VALUE_LEN + ID_LEN].copy_from_slice(&id_from_value(*value));
                self.set.vea.has_prefix(&prefix)
            }),
            (None, None, Some(v), false, true, false) => proposals.retain(|value| {
                let mut prefix = [0u8; VALUE_LEN + ID_LEN];
                prefix[0..VALUE_LEN].copy_from_slice(&v[..]);
                prefix[VALUE_LEN..VALUE_LEN + ID_LEN].copy_from_slice(&id_from_value(*value));
                self.set.vae.has_prefix(&prefix)
            }),
            (None, Some(a), Some(v), true, false, false) => proposals.retain(|value: &[u8; 32]| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v);
                prefix[ID_LEN + VALUE_LEN..ID_LEN + VALUE_LEN + ID_LEN]
                    .copy_from_slice(&id_from_value(*value));
                self.set.ave.has_prefix(&prefix)
            }),
            (Some(e), None, Some(v), false, true, false) => proposals.retain(|value: &[u8; 32]| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v);
                prefix[ID_LEN + VALUE_LEN..ID_LEN + VALUE_LEN + ID_LEN]
                    .copy_from_slice(&id_from_value(*value));
                self.set.eva.has_prefix(&prefix)
            }),
            (Some(e), Some(a), None, false, false, true) => proposals.retain(|value: &[u8; 32]| {
                let mut prefix = [0u8; ID_LEN + ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&a);
                prefix[ID_LEN + ID_LEN..ID_LEN + ID_LEN + VALUE_LEN].copy_from_slice(value);
                self.set.eav.has_prefix(&prefix)
            }),
            _ => panic!("invalid trible constraint state"),
        }
    }
}
