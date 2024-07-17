use std::collections::HashSet;

use super::*;

pub struct SetConstraint<'a, T>
{
    variable: Variable<T>,
    set: &'a HashSet<Value<T>>,
}

impl<'a, T> SetConstraint<'a, T>
{
    pub fn new(variable: Variable<T>, set: &'a HashSet<Value<T>>) -> Self {
        SetConstraint { variable, set }
    }
}

impl<'a, T> Constraint<'a> for SetConstraint<'a, T>
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

    fn propose(&self, _variable: VariableId, _binding: &Binding) -> Vec<RawValue> {
        self.set.iter().map(|v| v.bytes).collect()
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.retain(|v| self.set.contains(&Value::new(*v)));
    }
}

impl<'a, T> ContainsConstraint<'a, T> for HashSet<Value<T>>
where T: 'a
{
    type Constraint = SetConstraint<'a, T>;

    fn has(&'a self, v: Variable<T>) -> Self::Constraint {
        SetConstraint::new(v, self)
    }
}
