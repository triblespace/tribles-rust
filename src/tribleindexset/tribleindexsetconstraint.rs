use core::panic;

use crate::id::{id_from_value, id_into_value, ID_LEN};
use crate::query::{Binding, Constraint, Variable, VariableId, VariableSet};
use crate::trible::Trible;
use crate::tribleindexset::TribleIndexSet;
use crate::value::{schemas::genid::GenId, RawValue, ValueSchema, VALUE_LEN};

pub struct TribleIndexSetConstraint {
    variable_e: VariableId,
    variable_a: VariableId,
    variable_v: VariableId,
    set: TribleIndexSet,
}

impl TribleIndexSetConstraint {
    pub fn new<V: ValueSchema>(
        variable_e: Variable<GenId>,
        variable_a: Variable<GenId>,
        variable_v: Variable<V>,
        set: TribleIndexSet,
    ) -> Self {
        TribleIndexSetConstraint {
            variable_e: variable_e.index,
            variable_a: variable_a.index,
            variable_v: variable_v.index,
            set,
        }
    }
}

impl<'a> Constraint<'a> for TribleIndexSetConstraint {
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e);
        variables.set(self.variable_a);
        variables.set(self.variable_v);
        variables
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable_e == variable || self.variable_a == variable || self.variable_v == variable
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        let e_var = self.variable_e == variable;
        let a_var = self.variable_a == variable;
        let v_var = self.variable_v == variable;

        let e_bound = if let Some(e) = binding.get(self.variable_e) {
            let Some(e) = id_from_value(&e) else {
                return 0;
            };
            Some(e)
        } else {
            None
        };
        let a_bound = if let Some(a) = binding.get(self.variable_a) {
            let Some(a) = id_from_value(&a) else {
                return 0;
            };
            Some(a)
        } else {
            None
        };
        let v_bound = binding.get(self.variable_v);

        (match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => self.set.eav.len(),
            (None, None, None, false, true, false) => self.set.aev.len(),
            (None, None, None, false, false, true) => self.set.vea.len(),
            (Some(e), None, None, false, true, false) => {
                Trible::new_raw_values(e, [0; 32], [0; 32]);
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
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(v);
                self.set.ave.segmented_len(&prefix)
            }
            (Some(e), None, Some(v), false, true, false) => {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(v);
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

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<RawValue> {
        let e_var = self.variable_e == variable;
        let a_var = self.variable_a == variable;
        let v_var = self.variable_v == variable;

        let e_bound = if let Some(e) = binding.get(self.variable_e) {
            let Some(e) = id_from_value(&e) else {
                return vec![];
            };
            Some(e)
        } else {
            None
        };
        let a_bound = if let Some(a) = binding.get(self.variable_a) {
            let Some(a) = id_from_value(&a) else {
                return vec![];
            };
            Some(a)
        } else {
            None
        };
        let v_bound = binding.get(self.variable_v);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => {
                let mut r = vec![];
                self.set
                    .eav
                    .infixes(&[0; 0], &mut |e: &[u8; 16]| r.push(id_into_value(e)));
                r
            }
            (None, None, None, false, true, false) => {
                let mut r = vec![];
                self.set
                    .aev
                    .infixes(&[0; 0], &mut |a: &[u8; 16]| r.push(id_into_value(a)));
                r
            }
            (None, None, None, false, false, true) => {
                let mut r = vec![];
                self.set
                    .vea
                    .infixes(&[0; 0], &mut |&v: &[u8; 32]| r.push(v));
                r
            }

            (Some(e), None, None, false, true, false) => {
                let mut r = vec![];
                self.set
                    .eav
                    .infixes(&e, &mut |a: &[u8; 16]| r.push(id_into_value(a)));
                r
            }
            (Some(e), None, None, false, false, true) => {
                let mut r = vec![];
                self.set.eva.infixes(&e, &mut |&v: &[u8; 32]| r.push(v));
                r
            }

            (None, Some(a), None, true, false, false) => {
                let mut r = vec![];
                self.set
                    .aev
                    .infixes(&a, &mut |e: &[u8; 16]| r.push(id_into_value(e)));
                r
            }
            (None, Some(a), None, false, false, true) => {
                let mut r = vec![];
                self.set.ave.infixes(&a, &mut |&v: &[u8; 32]| r.push(v));
                r
            }

            (None, None, Some(v), true, false, false) => {
                let mut r = vec![];
                self.set
                    .vea
                    .infixes(&v, &mut |e: &[u8; 16]| r.push(id_into_value(e)));
                r
            }
            (None, None, Some(v), false, true, false) => {
                let mut r = vec![];
                self.set
                    .vae
                    .infixes(&v, &mut |a: &[u8; 16]| r.push(id_into_value(a)));
                r
            }
            (None, Some(a), Some(v), true, false, false) => {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v[..]);
                let mut r = vec![];
                self.set
                    .ave
                    .infixes(&prefix, &mut |e: &[u8; 16]| r.push(id_into_value(e)));
                r
            }
            (Some(e), None, Some(v), false, true, false) => {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v[..]);
                let mut r = vec![];
                self.set
                    .eva
                    .infixes(&prefix, &mut |a: &[u8; 16]| r.push(id_into_value(a)));
                r
            }
            (Some(e), Some(a), None, false, false, true) => {
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&a[..]);
                let mut r = vec![];
                self.set
                    .eav
                    .infixes(&prefix, &mut |&v: &[u8; 32]| r.push(v));
                r
            }
            _ => panic!(),
        }
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let e_var = self.variable_e == variable;
        let a_var = self.variable_a == variable;
        let v_var = self.variable_v == variable;

        let e_bound = if let Some(e) = binding.get(self.variable_e) {
            let Some(e) = id_from_value(&e) else {
                proposals.clear();
                return;
            };
            Some(e)
        } else {
            None
        };
        let a_bound = if let Some(a) = binding.get(self.variable_a) {
            let Some(a) = id_from_value(&a) else {
                proposals.clear();
                return;
            };
            Some(a)
        } else {
            None
        };
        let v_bound = binding.get(self.variable_v);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => proposals.retain(|value| {
                let Some(id) = id_from_value(value) else {
                    return false;
                };
                self.set.eav.has_prefix(&id)
            }),
            (None, None, None, false, true, false) => proposals.retain(|value| {
                let Some(id) = id_from_value(value) else {
                    return false;
                };
                self.set.aev.has_prefix(&id)
            }),
            (None, None, None, false, false, true) => {
                proposals.retain(|value| self.set.vea.has_prefix(value))
            }
            (Some(e), None, None, false, true, false) => proposals.retain(|value| {
                let Some(id) = id_from_value(value) else {
                    return false;
                };
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&id);
                self.set.eav.has_prefix(&prefix)
            }),
            (Some(e), None, None, false, false, true) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(value);
                self.set.eva.has_prefix(&prefix)
            }),
            (None, Some(a), None, true, false, false) => proposals.retain(|value| {
                let Some(id) = id_from_value(value) else {
                    return false;
                };
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&id);
                self.set.aev.has_prefix(&prefix)
            }),
            (None, Some(a), None, false, false, true) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(value);
                self.set.ave.has_prefix(&prefix)
            }),
            (None, None, Some(v), true, false, false) => proposals.retain(|value| {
                let Some(id) = id_from_value(value) else {
                    return false;
                };
                let mut prefix = [0u8; VALUE_LEN + ID_LEN];
                prefix[0..VALUE_LEN].copy_from_slice(&v[..]);
                prefix[VALUE_LEN..VALUE_LEN + ID_LEN].copy_from_slice(&id);
                self.set.vea.has_prefix(&prefix)
            }),
            (None, None, Some(v), false, true, false) => proposals.retain(|value| {
                let Some(id) = id_from_value(value) else {
                    return false;
                };
                let mut prefix = [0u8; VALUE_LEN + ID_LEN];
                prefix[0..VALUE_LEN].copy_from_slice(&v[..]);
                prefix[VALUE_LEN..VALUE_LEN + ID_LEN].copy_from_slice(&id);
                self.set.vae.has_prefix(&prefix)
            }),
            (None, Some(a), Some(v), true, false, false) => proposals.retain(|value: &[u8; 32]| {
                let Some(id) = id_from_value(value) else {
                    return false;
                };
                let mut prefix = [0u8; ID_LEN + VALUE_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(v);
                prefix[ID_LEN + VALUE_LEN..ID_LEN + VALUE_LEN + ID_LEN].copy_from_slice(&id);
                self.set.ave.has_prefix(&prefix)
            }),
            (Some(e), None, Some(v), false, true, false) => proposals.retain(|value: &[u8; 32]| {
                let Some(id) = id_from_value(value) else {
                    return false;
                };
                let mut prefix = [0u8; ID_LEN + VALUE_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(v);
                prefix[ID_LEN + VALUE_LEN..ID_LEN + VALUE_LEN + ID_LEN].copy_from_slice(&id);
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

#[cfg(test)]
mod tests {
    use crate::{
        find,
        id::rngid,
        query::{TriblePattern, Variable},
        trible::Trible,
        tribleindexset::TribleIndexSet,
        value::{schemas::UnknownValue, Value},
    };

    #[test]
    fn constant() {
        let mut set = TribleIndexSet::new();
        set.insert(&Trible::new(
            &rngid(),
            &rngid(),
            &Value::<UnknownValue>::new([0; 32]),
        ));

        let q = find!(
            ctx,
            (e: Value<_>, a: Value<_>, v: Value<_>),
            set.pattern(e, a, v as Variable<UnknownValue>)
        );
        let r: Vec<_> = q.collect();

        assert_eq!(1, r.len())
    }
}
