use crate::{patch::{PATCH, IdentityOrder, SingleSegmentation}, trible::{VALUE_LEN, Value}};

use super::{Variable, Constraint, VariableSet, VariableId, Binding, Constrain};


pub struct PatchConstraint<'a, T, V>
where V: Clone {
    variable: Variable<T>,
    patch: &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, V>,
}

impl<'a, T, V> PatchConstraint<'a, T, V>
where
    T: Eq + PartialEq + From<Value>,
    V: Clone
{
    pub fn new(variable: Variable<T>, patch: &'a PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, V>) -> Self {
        PatchConstraint { variable, patch }
    }
}

impl<'a, T, V> Constraint<'a> for PatchConstraint<'a, T, V>
where
    T: Eq + PartialEq + From<Value>,
    V: Clone
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
        self.patch.infixes(&[0; VALUE_LEN], 0, VALUE_LEN, |k| k)
    }

    fn confirm(&self, _variable: VariableId, _binding: Binding, proposals: &mut Vec<Value>) {
        proposals.retain(|v| self.patch.has_prefix(v, VALUE_LEN));
    }
}

impl<'a, T, V> Constrain<'a, T> for PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, V>
where
    T: Eq + PartialEq + From<Value> + 'a,
    V: Clone + 'a
{
    type Constraint = PatchConstraint<'a, T, V>;

    fn constrain(&'a self, v: Variable<T>) -> Self::Constraint {
        PatchConstraint::new(v, self)
    }
}