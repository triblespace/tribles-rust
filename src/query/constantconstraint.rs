use super::*;

pub struct ConstantConstraint {
    variable: VariableId,
    constant: RawValue,
}

impl ConstantConstraint {
    pub fn new<T: ValueSchema>(variable: Variable<T>, constant: Value<T>) -> Self {
        ConstantConstraint {
            variable: variable.index,
            constant: constant.bytes,
        }
    }
}

impl<'a> Constraint<'a> for ConstantConstraint {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable)
    }

    fn estimate(&self, variable: VariableId, _binding: &Binding) -> Option<usize> {
        if self.variable == variable {
            Some(1)
        } else {
            None
        }
    }

    fn propose(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.push(self.constant);
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.retain(|v| *v == self.constant);
    }
}
