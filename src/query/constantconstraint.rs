use super::*;

pub struct ConstantConstraint {
    variable: VariableId,
    constant: Value,
}

impl<'a> Constraint<'a> for ConstantConstraint {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable)
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
