use crate::{
    query::{Binding, Constraint, Variable, VariableId, VariableSet},
    types::{id_into_value, Idlike, Value, Valuelike},
};

use super::*;

pub struct AttributeConstraint<'a, E, V>
where
    E: Idlike,
    V: Valuelike,
{
    variable_e: Variable<E>,
    variable_v: Variable<V>,
    attr: &'a Attribute<E, V>,
}

impl<'a, E, V> AttributeConstraint<'a, E, V>
where
    E: Idlike,
    V: Valuelike,
{
    pub fn new(
        variable_e: Variable<E>,
        variable_v: Variable<V>,
        attr: &'a Attribute<E, V>,
    ) -> Self {
        AttributeConstraint {
            variable_e,
            variable_v,
            attr,
        }
    }
}

impl<'a, E, V> Constraint<'a> for AttributeConstraint<'a, E, V>
where
    E: Idlike,
    V: Valuelike,
{
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e.index);
        variables.set(self.variable_v.index);
        variables
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable_e.index == variable || self.variable_v.index == variable
    }

    fn estimate(&self, variable: VariableId, binding: Binding) -> usize {
        let e_var = self.variable_e.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) => self.attr.ev.len(),
            (None, None, false, true) => self.attr.ve.len(),
            (Some(e), None, false, true) => self.attr.ev.get(&e[16..32]).map_or(0, |s| s.len()),
            (None, Some(v), true, false) => self.attr.ve.get(&v).map_or(0, |s| s.len()),
            _ => panic!(),
        }
    }

    fn propose(&self, variable: VariableId, binding: Binding) -> Vec<Value> {
        let e_var = self.variable_e.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) => self.attr.ev.keys().copied().map(id_into_value).collect(),
            (None, None, false, true) => self.attr.ve.keys().copied().collect(),
            (Some(e), None, false, true) => self
                .attr
                .ev
                .get(&e[16..=31])
                .map_or(vec![], |s| s.iter().copied().collect()),
            (None, Some(v), true, false) => self
                .attr
                .ve
                .get(&v)
                .map_or(vec![], |s| s.iter().copied().map(id_into_value).collect()),
            _ => panic!(),
        }
    }

    fn confirm(&self, variable: VariableId, binding: Binding, proposals: &mut Vec<Value>) {
        let e_var = self.variable_e.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) => {
                proposals.retain(|e| self.attr.ev.contains_key(&e[16..=31]))
            }
            (None, None, false, true) => proposals.retain(|v| self.attr.ve.contains_key(v)),
            (Some(e), None, false, true) => {
                if let Some(vs) = self.attr.ev.get(&e[16..=31]) {
                    proposals.retain(|v| vs.contains(v));
                } else {
                    proposals.clear()
                }
            }
            (None, Some(v), true, false) => {
                if let Some(vs) = self.attr.ve.get(&v) {
                    proposals.retain(|e| vs.contains(&e[16..=31]));
                } else {
                    proposals.clear()
                }
            }
            _ => panic!(),
        }
    }
}
