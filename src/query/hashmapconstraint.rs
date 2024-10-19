use std::collections::HashMap;

use crate::id::{id_from_value, id_into_value, RawId};
use crate::query::{Binding, Constraint, ContainsConstraint, Variable, VariableId, VariableSet};
use crate::value::{schemas::genid::GenId, RawValue};

pub struct KeysConstraint<'a, T> {
    variable: Variable<GenId>,
    map: &'a HashMap<RawId, T>,
}

impl<'a, T> KeysConstraint<'a, T> {
    pub fn new(variable: Variable<GenId>, map: &'a HashMap<RawId, T>) -> Self {
        KeysConstraint { variable, map }
    }
}

impl<'a, T> Constraint<'a> for KeysConstraint<'a, T> {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }

    fn estimate(&self, _variable: VariableId, _binding: &Binding) -> usize {
        self.map.capacity()
    }

    fn propose(&self, _variable: VariableId, _binding: &Binding) -> Vec<RawValue> {
        self.map.keys().map(|id| id_into_value(id)).collect()
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.retain(|v| {
            if let Some(id) = id_from_value(v) {
                self.map.contains_key(&id)
            } else {
                false
            }
        });
    }
}

impl<'a, T> ContainsConstraint<'a, GenId> for HashMap<RawId, T>
where
    T: 'a,
{
    type Constraint = KeysConstraint<'a, T>;

    fn has(&'a self, v: Variable<GenId>) -> Self::Constraint {
        KeysConstraint::new(v, self)
    }
}
