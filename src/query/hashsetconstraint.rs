use std::{collections::HashSet, fmt::Debug, hash::Hash};

use super::*;

pub struct SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + Valuelike + Debug,
{
    variable: Variable<T>,
    set: &'a HashSet<T>,
}

impl<'a, T> SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + Valuelike + Debug,
{
    pub fn new(variable: Variable<T>, set: &'a HashSet<T>) -> Self {
        SetConstraint { variable, set }
    }
}

impl<'a, T> Constraint<'a> for SetConstraint<'a, T>
where
    T: Eq + PartialEq + Hash + Valuelike + Debug,
{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }

    fn estimate(&self, _variable: VariableId, _binding: &Binding) -> usize {
        self.set.len()
    }

    fn propose(&self, _variable: VariableId, _binding: &Binding) -> Vec<Value> {
        self.set.iter().map(|v| Valuelike::into_value(v)).collect()
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<Value>) {
        proposals.retain(|v| T::from_value(*v).map_or(false, |v| self.set.contains(&v)));
    }
}

impl<'a, T> ContainsConstraint<'a, T> for HashSet<T>
where
    T: Eq + PartialEq + Hash + Valuelike + Debug + 'a,
{
    type Constraint = SetConstraint<'a, T>;

    fn has(&'a self, v: Variable<T>) -> Self::Constraint {
        SetConstraint::new(v, self)
    }
}
