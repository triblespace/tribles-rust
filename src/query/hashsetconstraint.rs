use std::collections::HashSet;

use crate::value::{FromValue, ToValue};

use super::*;

pub struct SetConstraint<'a, S: ValueSchema, T> {
    variable: Variable<S>,
    set: &'a HashSet<T>,
}

impl<'a, S: ValueSchema, T> SetConstraint<'a, S, T> {
    pub fn new(variable: Variable<S>, set: &'a HashSet<T>) -> Self {
        SetConstraint { variable, set }
    }
}

impl<'a, S: ValueSchema, T> Constraint<'a> for SetConstraint<'a, S, T>
where T: 'a + std::cmp::Eq + std::hash::Hash,
      for<'b> &'b T: ToValue<S> + FromValue<'b, S> {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }

    fn estimate(&self, _variable: VariableId, _binding: &Binding) -> usize {
        self.set.capacity()
    }

    fn propose(&self, _variable: VariableId, _binding: &Binding) -> Vec<RawValue> {
        self.set.iter().map(|v| ToValue::to_value(v).bytes).collect()
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.retain(|v| self.set.contains(FromValue::from_value(Value::<S>::transmute_raw(v))));
    }
}

impl<'a, S: ValueSchema, T> ContainsConstraint<'a, S> for HashSet<T>
where
    T: 'a + std::cmp::Eq + std::hash::Hash,
    for<'b> &'b T: ToValue<S> + FromValue<'b, S> 
{
    type Constraint = SetConstraint<'a, S, T>;

    fn has(&'a self, v: Variable<S>) -> Self::Constraint {
        SetConstraint::new(v, self)
    }
}
