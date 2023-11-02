use super::*;

pub struct ConstantConstraint<T> {
    variables: VariableSet,
    constant: Value,
    phantom: PhantomData<T>
}

impl<T> ConstantConstraint<T> {
    pub fn new(variable: Variable<T>, constant: &T) -> Self
    where
        for<'b> &'b T: Into<Value>,
    {
        ConstantConstraint {
            phantom: PhantomData,
            variables: VariableSet::new_singleton(variable.index),
            constant: constant.into(),
        }
    }
}

impl<'a, T> Constraint<'a> for ConstantConstraint<T> {
    fn variables(&self) -> VariableSet {
        self.variables
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
