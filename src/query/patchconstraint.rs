use crate::{
    patch::{IdentityOrder, SingleSegmentation, PATCH},
    types::{Value, Valuelike, VALUE_LEN},
};

use super::{Binding, Constrain, Constraint, Variable, VariableId, VariableSet};

pub struct PatchConstraint<'a, T, V>
where
    V: Clone,
{
    variable: Variable<T>,
    patch: &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, V>,
}

impl<'a, T, V> PatchConstraint<'a, T, V>
where
    T: Eq + PartialEq + Valuelike,
    V: Clone,
{
    pub fn new(
        variable: Variable<T>,
        patch: &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, V>,
    ) -> Self {
        PatchConstraint { variable, patch }
    }
}

impl<'a, T, V> Constraint<'a> for PatchConstraint<'a, T, V>
where
    T: Eq + PartialEq + Valuelike,
    V: Clone,
{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }

    fn estimate(&self, _variable: VariableId, _binding: Binding) -> usize {
        self.patch.len() as usize
    }

    fn propose(&self, _variable: VariableId, _binding: Binding) -> Vec<Value> {
        let mut r = vec![];
        self.patch
            .infixes::<0, VALUE_LEN, _>(&[0; 0], &mut |k| r.push(k));
        r
    }

    fn confirm(&self, _variable: VariableId, _binding: Binding, proposals: &mut Vec<Value>) {
        proposals.retain(|v| self.patch.has_prefix(v));
    }
}

impl<'a, T, V> Constrain<'a, T> for PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, V>
where
    T: Eq + PartialEq + Valuelike + 'a,
    V: Clone + 'a,
{
    type Constraint = PatchConstraint<'a, T, V>;

    fn constrain(&'a self, v: Variable<T>) -> Self::Constraint {
        PatchConstraint::new(v, self)
    }
}
