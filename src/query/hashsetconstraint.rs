use std::{collections::HashSet, fmt::Debug, hash::Hash};

use super::*;

pub struct SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + From<Value> + Debug,
    for<'b> &'b T: Into<Value>,
{
    variable: Variable<T>,
    set: &'a HashSet<T>,
}

impl<'a, T> SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + From<Value> + Debug,
    for<'b> &'b T: Into<Value>,
{
    pub fn new(variable: Variable<T>, set: &'a HashSet<T>) -> Self {
        SetConstraint { variable, set }
    }
}

impl<'a, T> Constraint<'a> for SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + From<Value> + Debug,
    for<'b> &'b T: Into<Value>,
{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }
    
    fn estimate(&self, _variable: VariableId, _binding: Binding) -> usize {
        self.set.len()
    }

    fn propose(&self, _variable: VariableId, _binding: Binding) -> Vec<Value> {
        self.set.iter().map(|v| v.into()).collect()
    }

    fn confirm(&self, _variable: VariableId, _binding: Binding, proposals: &mut Vec<Value>) {
        proposals.retain(|v| self.set.contains(&((*v).into())));
    }
}

impl<'a, T> Constrain<'a, T> for HashSet<T>
where
    T: Eq + PartialEq + Hash + From<Value> + Debug + 'a,
    for<'b> &'b T: Into<Value>,
{
    type Constraint = SetConstraint<'a, T>;

    fn constrain(&'a self, v: Variable<T>) -> Self::Constraint {
        SetConstraint::new(v, self)
    }
}
