use crate::{
    query::{Binding, Constraint, Variable, VariableId, VariableSet},
    trible::Trible,
};

use super::*;

pub struct AttrConstraint<'a, Id, Value> {
    variable_e: Variable<Id>,
    variable_v: Variable<Value>,
    attr: &'a Attribute<Id, Value>,
}

impl<'a, Id, Value> AttrConstraint<'a, Id, Value> {
    pub fn new(
        variable_e: Variable<Id>,
        variable_v: Variable<Value>,
        attr: &'a Attribute<Id, Value>,
    ) -> Self {
        AttrConstraint {
            variable_e,
            variable_v,
            attr,
        }
    }
}

impl<'a, Id, Value> Constraint<'a> for AttrConstraint<'a, Id, Value> {
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e.index);
        variables.set(self.variable_v.index);
        variables
    }

    fn estimate(&self, variable: Variable<T>, binding: Binding) -> usize {
        let e_bound = binding.bound.get(self.variable_e.index);
        let v_bound = binding.bound.get(self.variable_v.index);

        let e_var = self.variable_e.index == variable.index;
        let v_var = self.variable_v.index == variable.index;

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) => self.attr.ev.len(),
            (None, None, false, true) => self.attr.ve.len(),
            (Some(_), None, false, true) => {
                self.attr.ev.get(&trible.e()).map_or(0, |s| s.len())
            }
            (None, Some(_), true, false) => {
                self.attr.ve.get(&trible.v()).map_or(0, |s| s.len())
            }
            _ => panic!(),
        }
    }

    fn propose<T>(&self, variable: Variable<T>, binding: Binding) -> Vec<T> {
        let e_bound = binding.bound.get(self.variable_e.index);
        let v_bound = binding.bound.get(self.variable_v.index);

        let e_var = self.variable_e.index == variable.index;
        let v_var = self.variable_v.index == variable.index;

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) =>
                self.attr.ev.keys().copied().collect::<Vec<T>>(),
            (None, None, false, true) =>
                self.attr.ve.keys().copied().collect::<Vec<T>>(),
            (Some(e), None, false, true) => self.attr.ev.get(e)
                .map_or(vec![], |s| s.iter().copied().collect::<Vec<T>>()),
            (None, Some(v), true, false) => self.attr.ve.get(v)
                .map_or(vec![], |s| s.iter().copied().collect::<Vec<T>>()),
            _ => panic!(),
        }
    }

    fn confirm<T>(&self, variable: Variable<T>, binding: Binding, proposals: &mut Vec<T>) {
        let e_bound = binding.bound.get(self.variable_e.index);
        let v_bound = binding.bound.get(self.variable_v.index);

        let e_var = self.variable_e.index == variable.index;
        let v_var = self.variable_v.index == variable.index;

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) =>
                proposals.retain(|e| self.attr.ev.contains_key(e)),
            (None, None, false, true) =>
                proposals.retain(|v| self.attr.ve.contains_key(v)),
            (Some(e), None, false, true) => {
                if let Some(vs) = self.set.ev.get(e) {
                    proposals.retain(|v| vs.contains(v));
                } else {
                    proposals.clear()
                }
            }
            (None, Some(v), true, false) => {
                if let Some(vs) = self.set.ve.get(v) {
                    proposals.retain(|e| vs.contains(e));
                } else {
                    proposals.clear()
                }
            }
            _ => panic!(),
        }
    }
}
