use std::{collections::HashSet, hash::Hash, rc::Rc};

use std::collections::hash_set::Iter;

use super::*;

pub struct SetConstraint<'a, T>
where
    T: Eq + PartialEq + Copy + Hash + Into<Value> + From<Value>,
{
    variable: VariableId,
    set: &'a HashSet<T>,
}

impl<'a, T> SetConstraint<'a, T>
where
    T: Eq + PartialEq + Copy + Hash + Into<Value> + From<Value>,
{
    fn new(variable: VariableId, set: &'a HashSet<T>) -> Self {
        SetConstraint { variable, set }
    }
}

impl<'a, T> Constraint<'a> for SetConstraint<'a, T>
where
    T: Eq + PartialEq + Copy + Hash + Into<Value> + From<Value>,
{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable)
    }

    fn estimate(&self, _variable: VariableId) -> usize {
        self.set.len()
    }

    fn propose(&self, _variable: VariableId, _binding: Binding) -> Box<dyn Iterator<Item = Value> + 'a> {
        let iter: Iter<'a, _> = self.set.iter();
        Box::new(iter.map(|v| (*v).into()))
    }

    fn confirm(&self, _variable: VariableId, value: Value, _binding: Binding) -> bool {
        self.set.contains(&(value.into()))
    }
}
