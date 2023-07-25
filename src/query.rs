mod constantconstraint;
mod hashsetconstraint;
mod intersectionconstraint;

use std::marker::PhantomData;

use constantconstraint::*;
use hashsetconstraint::*;
use intersectionconstraint::*;

use crate::namespace::*;

use crate::bitset::ByteBitset;

pub type VariableId = u8;
pub type VariableSet = ByteBitset;

#[derive(Debug)]
pub struct Variable<T> {
    pub index: VariableId,
    typed: PhantomData<T>,
}

impl<T> Copy for Variable<T> {}

impl<T> Clone for Variable<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Variable<T> {
    pub fn new(index: VariableId) -> Self {
        Variable {
            index,
            typed: PhantomData,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Binding {
    pub bound: VariableSet,
    values: [Value; 256],
}

impl Binding {
    pub fn set(&mut self, variable: VariableId, value: Value) {
        self.values[variable as usize] = value;
        self.bound.set(variable);
    }

    pub fn unset(&mut self, variable: VariableId) {
        self.bound.unset(variable);
    }

    pub fn get(&self, variable: VariableId) -> Option<Value> {
        if self.bound.is_set(variable) {
            Some(self.values[variable as usize])
        } else {
            None
        }
    }
}

impl Default for Binding {
    fn default() -> Self {
        Self {
            bound: ByteBitset::new_empty(),
            values: [[0; 32]; 256],
        }
    }
}

pub trait Constraint<'a> {
    fn variables(&self) -> VariableSet;
    fn estimate(&self, variable: VariableId, binding: Binding) -> usize;
    fn propose(&self, variable: VariableId, binding: Binding) -> Vec<Value>;
    fn confirm(&self, variable: VariableId, value: Value, binding: Binding) -> bool;
}

struct Query<C> {
    constraint: C,
    binding: Binding,
    variables: VariableSet,
    variable_stack: [u8; 256],
    value_stack: [Vec<Value>; 256],
    stack_depth: isize,
}

impl<'a, C: Constraint<'a>> Query<C> {
    fn new(constraint: C) -> Self {
        let variables = constraint.variables();
        Query {
            constraint,
            binding: Default::default(),
            variables,
            variable_stack: [0; 256],
            value_stack: std::array::from_fn(|_| vec![]),
            stack_depth: -1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Search {
    Vertical,
    Horizontal,
    Backtrack,
}

impl<'a, C: Constraint<'a>> Iterator for Query<C> {
    // we will be counting with usize
    type Item = Binding;

    // next() is the only required method
    fn next(&mut self) -> Option<Self::Item> {
        let mut mode = if self.stack_depth == -1 {
            Search::Vertical
        } else {
            Search::Horizontal
        };

        loop {
            match mode {
                Search::Vertical => {
                    if let Some(next_variable) = {
                        let unbound_variables = self.variables.subtract(self.binding.bound);
                        let next_variable = unbound_variables
                            .into_iter()
                            .min_by_key(|v| self.constraint.estimate(*v, self.binding));
                        next_variable
                    } {
                        self.stack_depth += 1;
                        self.variable_stack[self.stack_depth as usize] = next_variable;
                        self.value_stack[self.stack_depth as usize] =
                            self.constraint.propose(next_variable, self.binding);
                        mode = Search::Horizontal;
                    } else {
                        return Some(self.binding.clone());
                    }
                }
                Search::Horizontal => {
                    if let Some(assignment) = self.value_stack[self.stack_depth as usize].pop() {
                        self.binding
                            .set(self.variable_stack[self.stack_depth as usize], assignment);
                        mode = Search::Vertical;
                    } else {
                        mode = Search::Backtrack;
                    }
                }
                Search::Backtrack => {
                    self.binding
                        .unset(self.variable_stack[self.stack_depth as usize]);
                    self.stack_depth -= 1;
                    if self.stack_depth == -1 {
                        return None;
                    }
                    mode = Search::Horizontal;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    use crate::types::shortstring::ShortString;

    #[test]
    fn and_set() {
        let mut books = HashSet::new();
        let mut movies = HashSet::new();

        books.insert(ShortString::new("LOTR".into()).unwrap());
        books.insert(ShortString::new("Dragonrider".into()).unwrap());
        books.insert(ShortString::new("Highlander".into()).unwrap());

        movies.insert(ShortString::new("LOTR".into()).unwrap());
        movies.insert(ShortString::new("Highlander".into()).unwrap());

        let a = Variable::new(0);
        let b = Variable::new(1);

        let inter: Vec<Binding> = Query::new(IntersectionConstraint::new(vec![
            Box::new(SetConstraint::new(a, &books)),
            Box::new(SetConstraint::new(a, &movies)),
        ]))
        .collect();

        assert_eq!(inter.len(), 2);

        let cross: Vec<Binding> = Query::new(IntersectionConstraint::new(vec![
            Box::new(SetConstraint::new(a, &books)),
            Box::new(SetConstraint::new(b, &movies)),
        ]))
        .collect();

        assert_eq!(cross.len(), 6);
    }
}
