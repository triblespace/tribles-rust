use std::{collections::HashSet, ops::Deref, rc::Rc, sync::Arc};

use crate::value::{FromValue, ToValue};

use super::*;

pub struct SetConstraint<S: ValueSchema, R, T>
where
    R: Deref<Target = HashSet<T>>,
{
    variable: Variable<S>,
    set: R,
}

impl<S: ValueSchema, R, T> SetConstraint<S, R, T>
where
    R: Deref<Target = HashSet<T>>,
{
    pub fn new(variable: Variable<S>, set: R) -> Self {
        SetConstraint { variable, set }
    }
}

impl<'a, S: ValueSchema, R, T> Constraint<'a> for SetConstraint<S, R, T>
where
    T: 'a + std::cmp::Eq + std::hash::Hash + for<'b> FromValue<'b, S>,
    for<'b> &'b T: ToValue<S>,
    R: Deref<Target = HashSet<T>>,
{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }

    fn estimate(&self, _variable: VariableId, _binding: &Binding) -> usize {
        self.set.capacity()
    }

    fn propose(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.extend(self.set.iter().map(|v| ToValue::to_value(v).raw));
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.retain(|v| {
            let t = FromValue::from_value(Value::<S>::transmute_raw(v));
            self.set.contains(&t)
        });
    }
}

impl<'a, S: ValueSchema, T> ContainsConstraint<'a, S> for &'a HashSet<T>
where
    T: 'a + std::cmp::Eq + std::hash::Hash + for<'b> FromValue<'b, S>,
    for<'b> &'b T: ToValue<S>,
{
    type Constraint = SetConstraint<S, Self, T>;

    fn has(self, v: Variable<S>) -> Self::Constraint {
        SetConstraint::new(v, self)
    }
}

impl<'a, S: ValueSchema, T> ContainsConstraint<'a, S> for Rc<HashSet<T>>
where
    T: 'a + std::cmp::Eq + std::hash::Hash + for<'b> FromValue<'b, S>,
    for<'b> &'b T: ToValue<S>,
{
    type Constraint = SetConstraint<S, Self, T>;

    fn has(self, v: Variable<S>) -> Self::Constraint {
        SetConstraint::new(v, self)
    }
}

impl<'a, S: ValueSchema, T> ContainsConstraint<'a, S> for Arc<HashSet<T>>
where
    T: 'a + std::cmp::Eq + std::hash::Hash + for<'b> FromValue<'b, S>,
    for<'b> &'b T: ToValue<S>,
{
    type Constraint = SetConstraint<S, Self, T>;

    fn has(self, v: Variable<S>) -> Self::Constraint {
        SetConstraint::new(v, self)
    }
}
