use std::{collections::HashSet, hash::Hash, fmt::Debug};

use super::*;

pub struct SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + From<Value> + Debug,
    for<'b> &'b T: Into<Value>,
{
    variable: VariableId,
    set: &'a HashSet<T>,
}

impl<'a, T> SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + From<Value> + Debug,
    for<'b> &'b T: Into<Value>,
{
    pub fn new(variable: VariableId, set: &'a HashSet<T>) -> Self {
        SetConstraint { variable, set }
    }
}

impl<'a, T> Constraint<'a> for SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + From<Value> + Debug,
    for<'b> &'b T: Into<Value>,
{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable)
    }

    fn estimate(&self, _variable: VariableId) -> usize {
        self.set.len()
    }

    fn propose(&self, _variable: VariableId, _binding: Binding) -> Box<Vec<Value>> {
        Box::new(self.set.iter().map(|v| v.into()).collect())
    }

    fn confirm(&self, _variable: VariableId, value: Value, _binding: Binding) -> bool {
        self.set.contains(&(value.into()))
    }
}
