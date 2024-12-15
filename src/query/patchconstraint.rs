use crate::{
    id::{id_into_value, ID_LEN},
    patch::{IdentityOrder, SingleSegmentation, PATCH},
    value::{RawValue, ValueSchema, VALUE_LEN},
};

use super::{Binding, Constraint, ContainsConstraint, Variable, VariableId, VariableSet};

pub struct PatchValueConstraint<'a, T: ValueSchema> {
    variable: Variable<T>,
    patch: &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation>,
}

impl<'a, T: ValueSchema> PatchValueConstraint<'a, T> {
    pub fn new(
        variable: Variable<T>,
        patch: &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation>,
    ) -> Self {
        PatchValueConstraint { variable, patch }
    }
}

impl<'a, S: ValueSchema> Constraint<'a> for PatchValueConstraint<'a, S> {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn estimate(&self, variable: VariableId, _binding: &Binding) -> Option<usize> {
        if self.variable.index == variable {
            Some(self.patch.len() as usize)
        } else {
            None
        }
    }

    fn propose(&self, variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        if self.variable.index == variable {
            self.patch
                .infixes(&[0; 0], &mut |&k: &[u8; 32]| proposals.push(k));
        }
    }

    fn confirm(&self, variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        if self.variable.index == variable {
            proposals.retain(|v| self.patch.has_prefix(v));
        }
    }
}

impl<'a, S: ValueSchema> ContainsConstraint<'a, S>
    for &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation>
{
    type Constraint = PatchValueConstraint<'a, S>;

    fn has(self, v: Variable<S>) -> Self::Constraint {
        PatchValueConstraint::new(v, self)
    }
}

pub struct PatchIdConstraint<S>
where
    S: ValueSchema,
{
    variable: Variable<S>,
    patch: PATCH<ID_LEN, IdentityOrder, SingleSegmentation>,
}

impl<'a, S> PatchIdConstraint<S>
where
    S: ValueSchema,
{
    pub fn new(
        variable: Variable<S>,
        patch: PATCH<ID_LEN, IdentityOrder, SingleSegmentation>,
    ) -> Self {
        PatchIdConstraint { variable, patch }
    }
}

impl<'a, S> Constraint<'a> for PatchIdConstraint<S>
where
    S: ValueSchema,
{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn estimate(&self, variable: VariableId, _binding: &Binding) -> Option<usize> {
        if self.variable.index == variable {
            Some(self.patch.len() as usize)
        } else {
            None
        }
    }

    fn propose(&self, variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        if self.variable.index == variable {
            self.patch.infixes(&[0; 0], &mut |id: &[u8; 16]| {
                proposals.push(id_into_value(id))
            });
        }
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.retain(|v| self.patch.has_prefix(v));
    }
}

impl<'a, S: ValueSchema> ContainsConstraint<'a, S>
    for PATCH<ID_LEN, IdentityOrder, SingleSegmentation>
{
    type Constraint = PatchIdConstraint<S>;

    fn has(self, v: Variable<S>) -> Self::Constraint {
        PatchIdConstraint::new(v, self)
    }
}
