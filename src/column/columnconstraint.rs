use crate::{
    id_into_value,
    query::{Binding, Constraint, Variable, VariableId, VariableSet},
    RawValue,
};

use super::*;

pub struct ColumnConstraint<'a> {
    variable_e: VariableId,
    variable_v: VariableId,
    column_ev: &'a HashMap<RawId, HashSet<RawValue>>,
    column_ve: &'a HashMap<RawValue, HashSet<RawId>>,
}

impl<'a> ColumnConstraint<'a> {
    pub fn new<V: ValueSchema>(
        variable_e: Variable<GenId>,
        variable_v: Variable<V>,
        column: &'a Column<V>,
    ) -> Self {
        ColumnConstraint {
            variable_e: variable_e.index,
            variable_v: variable_v.index,
            column_ev: &column.ev,
            column_ve: &column.ve,
        }
    }
}

impl<'a> Constraint<'a> for ColumnConstraint<'a> {
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e);
        variables.set(self.variable_v);
        variables
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable_e == variable || self.variable_v == variable
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        let e_var = self.variable_e == variable;
        let v_var = self.variable_v == variable;

        let e_bound = binding.get(self.variable_e);
        let v_bound = binding.get(self.variable_v);

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) => self.column_ev.len(),
            (None, None, false, true) => self.column_ve.len(),
            (Some(e), None, false, true) => self.column_ev.get(&e[16..32]).map_or(0, |s| s.len()),
            (None, Some(v), true, false) => self.column_ve.get(v).map_or(0, |s| s.len()),
            _ => panic!(),
        }
    }

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<RawValue> {
        let e_var = self.variable_e == variable;
        let v_var = self.variable_v == variable;

        let e_bound = binding.get(self.variable_e);
        let v_bound = binding.get(self.variable_v);

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) => self.column_ev.keys().map(id_into_value).collect(),
            (None, None, false, true) => self.column_ve.keys().copied().collect(),
            (Some(e), None, false, true) => self
                .column_ev
                .get(&e[16..=31])
                .map_or(vec![], |s| s.iter().copied().collect()),
            (None, Some(v), true, false) => self
                .column_ve
                .get(v)
                .map_or(vec![], |s| s.iter().map(id_into_value).collect()),
            _ => panic!(),
        }
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let e_var = self.variable_e == variable;
        let v_var = self.variable_v == variable;

        let e_bound = binding.get(self.variable_e);
        let v_bound = binding.get(self.variable_v);

        match (e_bound, v_bound, e_var, v_var) {
            (None, None, true, false) => {
                proposals.retain(|e| self.column_ev.contains_key(&e[16..=31]))
            }
            (None, None, false, true) => proposals.retain(|v| self.column_ve.contains_key(v)),
            (Some(e), None, false, true) => {
                if let Some(vs) = self.column_ev.get(&e[16..=31]) {
                    proposals.retain(|v| vs.contains(v));
                } else {
                    proposals.clear()
                }
            }
            (None, Some(v), true, false) => {
                if let Some(vs) = self.column_ve.get(v) {
                    proposals.retain(|e| vs.contains(&e[16..=31]));
                } else {
                    proposals.clear()
                }
            }
            _ => panic!(),
        }
    }
}
