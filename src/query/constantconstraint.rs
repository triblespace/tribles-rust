use super::*;

pub struct ConstantConstraint<T> {
    variable: Variable<T>,
    constant: Value,
}

impl<T> ConstantConstraint<T> {
    pub fn new(variable: Variable<T>, constant: T) -> Self
    where
        T: Valuelike,
    {
        ConstantConstraint {
            variable,
            constant: constant.into_value(),
        }
    }
}

impl<'a, T> Constraint<'a> for ConstantConstraint<T> {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }

    fn estimate(&self, _variable: VariableId, _binding: Binding) -> usize {
        1
    }

    fn propose(&self, _variable: VariableId, _binding: Binding) -> Vec<Value> {
        vec![self.constant]
    }

    fn confirm(&self, _variable: VariableId, _binding: Binding, proposals: &mut Vec<Value>) {
        proposals.retain(|v| *v == self.constant);
    }
}
