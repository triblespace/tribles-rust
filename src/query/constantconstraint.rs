use super::*;

pub struct ConstantConstraint<T> {
    variable: Variable<T>,
    constant: Value,
}

impl<T> ConstantConstraint<T> {
    pub fn new(variable: Variable<T>, constant: &T) -> Self
    where for<'b> &'b T: Into<Value> {
        ConstantConstraint {
            variable,
            constant: constant.into()
        }
    }
}

impl<'a, T> Constraint<'a> for ConstantConstraint<T> {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn estimate(&self, _variable: VariableId, _binding: Binding) -> usize {
        1
    }

    fn propose(&self, _variable: VariableId, _binding: Binding) -> Vec<Value> {
        vec![self.constant]
    }

    fn confirm(&self, _variable: VariableId, value: Value, _binding: Binding) -> bool {
        value == self.constant
    }
}
