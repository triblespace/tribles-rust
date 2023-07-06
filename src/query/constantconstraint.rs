use super::*;

pub struct ConstantConstraint {
    variable: VariableId,
    constant: Value,
}

impl Constraint for ConstantConstraint {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable)
    }

    fn estimate(&self, _variable: VariableId) -> u64 {
        1
    }
    
    fn propose(&self, variable: VariableId, binding: Binding) -> Box<dyn Iterator<Item = Value>> {
        [self.constant].into()
    }

    fn confirm(&self, variable: VariableId, value: Value, binding: Binding) -> bool {
        value == self.constant
    }
}
