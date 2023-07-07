use super::*;

pub struct ConstantConstraint {
    variable: VariableId,
    constant: Value,
}

impl Constraint<'_> for ConstantConstraint {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable)
    }

    fn estimate(&self, _variable: VariableId) -> u64 {
        1
    }

    fn propose(
        &self,
        _variable: VariableId,
        _binding: Binding,
    ) -> Box<dyn Iterator<Item = Value> + '_> {
        Box::new(std::iter::once(self.constant))
    }

    fn confirm(&self, _variable: VariableId, value: Value, _binding: Binding) -> bool {
        value == self.constant
    }
}
