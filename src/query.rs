mod constantconstraint;
mod intersectionconstraint;

use constantconstraint::*;
use intersectionconstraint::*;

use crate::bitset::ByteBitset;

pub type Value = [u8; 32];
pub type VariableId = u8;
pub type VariableSet = ByteBitset;

#[derive(Copy, Clone)]
struct Binding {
    bound: VariableSet,
    values: [Value; 256],
}

impl Default for Binding {
    fn default() -> Self {
        Self {
            bound: ByteBitset::new_empty(),
            values: [[0; 32]; 256],
        }
    }
}

pub trait Constraint {
    fn variables(&self) -> VariableSet;
    fn estimate(&self, variable: VariableId) -> u64;
    fn propose(&self, variable: VariableId, binding: Binding) -> Box<dyn Iterator<Item = Value>>;
    fn confirm(&self, variable: VariableId, value: Value, binding: Binding) -> bool;
}
struct ConstraintIterator<C: Constraint> {
    constraint: C,
    binding: Binding,
    variables: VariableSet,
    variable_stack: [u8; 256],
    iterator_stack: [Option<Box<dyn Iterator<Item = Value>>>],
    stack_depth: isize,
}

impl<C: Constraint> ConstraintIterator<C> {
    fn new(constraint: C) -> Self {
        let variables = constraint.variables();
        ConstraintIterator {
            constraint,
            binding: Default::default(),
            variables,
            variable_stack: [0; 256],
            iterator_stack: [None; 256],
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

impl<C: Constraint> Iterator for ConstraintIterator<C> {
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
                            .ascending()
                            .min_by_key(|v| self.constraint.estimate(*v));
                        next_variable
                    } {
                        self.stack_depth += 1;
                        self.variable_stack[self.stack_depth] = next_variable;
                        self.iterator_stack[self.stack_depth] = Some(self.constraint.propose(next_variable));
                        mode = Search::Horizontal;
                    } else {
                        return Some(self.binding.clone());
                    }
                }
                Search::Horizontal => {
                    if let Some(assignment) = self.iterator_stack[self.depth].next() {
                        self.binding.set(self.variable_stack[self.depth], assignment);
                        mode = Search::Vertical;
                    } else {
                        mode = Search::Backtrack;
                    }
                }
                Search::Backtrack => {
                        self.binding.unset(self.variable_stack[self.depth]);
                        self.iterator_stack[self.depth] = None;
                        self.depth -= 1;
                        if self.depth == -1 {
                            return None;
                        }
                        mode = Search::Vertical;
                }
            }
        }
    }
}
