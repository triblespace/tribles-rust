use super::*;

pub struct ConstantConstraint<T> {
    variable: Variable<T>,
    constant: Value,
}

impl<T> ConstantConstraint<T> {
    pub fn new(variable: Variable<T>, constant: &T) -> Self
    where
        for<'b> &'b T: Into<Value>,
    {
        ConstantConstraint {
            variable,
            constant: constant.into(),
        }
    }
}

impl<'a, T> Constraint<'a> for ConstantConstraint<T> {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn estimate(&self, variable: VariableId, _binding: Binding) -> Option<usize> {
        if variable == self.variable.index {
            Some(1)
        } else {
            None
        }
    }

    fn propose(&self, _variable: VariableId, _binding: Binding) -> Vec<Value> {
        vec![self.constant]
    }

    fn confirm(&self, _variable: VariableId, _binding: Binding, proposals: &mut Vec<Value>) {
        proposals.retain(|v| *v == self.constant);
    }
}
