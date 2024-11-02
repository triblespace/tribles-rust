use std::collections::HashMap;

use crate::query::{Binding, Constraint, ContainsConstraint, Variable, VariableId, VariableSet};
use crate::value::{FromValue, ToValue, Value, ValueSchema};
use crate::value::RawValue;

pub struct KeysConstraint<'a, S: ValueSchema, K, V> {
    variable: Variable<S>,
    map: &'a HashMap<K, V>,
}

impl<'a, S: ValueSchema, K, V> KeysConstraint<'a, S, K, V> {
    pub fn new(variable: Variable<S>, map: &'a HashMap<K, V>) -> Self {
        KeysConstraint { variable, map }
    }
}

impl<'a, S: ValueSchema, K, V> Constraint<'a> for KeysConstraint<'a, S, K, V>
where K: 'a + std::cmp::Eq + std::hash::Hash,
      for<'b> &'b K: ToValue<S> + FromValue<'b, S>{
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable.index)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable.index == variable
    }

    fn estimate(&self, _variable: VariableId, _binding: &Binding) -> usize {
        self.map.capacity()
    }

    fn propose(&self, _variable: VariableId, _binding: &Binding) -> Vec<RawValue> {
        self.map.keys().map(|k| ToValue::to_value(k).bytes).collect()
    }

    fn confirm(&self, _variable: VariableId, _binding: &Binding, proposals: &mut Vec<RawValue>) {
        proposals.retain(|v| self.map.contains_key(FromValue::from_value(Value::<S>::transmute_raw(v))));
    }
}

impl<'a, S: ValueSchema, K, V> ContainsConstraint<'a, S> for HashMap<K, V>
where
    K: 'a + std::cmp::Eq + std::hash::Hash,
    for<'b> &'b K: ToValue<S> + FromValue<'b, S>,
    V: 'a
{
    type Constraint = KeysConstraint<'a, S, K, V>;

    fn has(&'a self, v: Variable<S>) -> Self::Constraint {
        KeysConstraint::new(v, self)
    }
}
