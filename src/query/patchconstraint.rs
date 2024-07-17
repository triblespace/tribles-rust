use crate::{
    patch::{IdentityOrder, SingleSegmentation, PATCH},
    RawValue, VALUE_LEN,
};

use super::{Binding, ContainsConstraint, Constraint, Variable, VariableId, VariableSet};

pub struct PatchConstraint<'a, T> {
    variable: Variable<T>,
    patch: &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation>,
}

impl<'a, T> PatchConstraint<'a, T>
{
    pub fn new(
        variable: Variable<T>,
        patch: &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation>,
    ) -> Self {
        PatchConstraint { variable, patch }
    }
}

impl<'a, T> Constraint<'a> for PatchConstraint<'a, T>
{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }

    fn estimate(&self, _variable: VariableId, _binding: &Binding) -> usize {
        self.patch.len() as usize
    }

    fn propose(&self, _variable: VariableId, _binding: &Binding) -> Vec<RawValue> {
        let mut r = vec![];
        self.patch
            .infixes::<0, VALUE_LEN, _>(&[0; 0], &mut |k| r.push(k));
        r
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.retain(|v| self.patch.has_prefix(v));
    }
}

impl<'a, T> ContainsConstraint<'a, T> for PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation> {
    type Constraint = PatchConstraint<'a, T>;

    fn has(&'a self, v: Variable<T>) -> Self::Constraint {
        PatchConstraint::new(v, self)
    }
}
