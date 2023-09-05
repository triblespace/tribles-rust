mod constantconstraint;
mod hashsetconstraint;
mod intersectionconstraint;

use std::marker::PhantomData;

pub use constantconstraint::*;
pub use hashsetconstraint::*;
pub use intersectionconstraint::*;

use crate::namespace::*;

use crate::bitset::ByteBitset;

pub type VariableId = u8;
pub type VariableSet = ByteBitset;

#[derive(Debug)]
pub struct VariableContext {
    pub next_index: VariableId,
}

impl VariableContext {
    pub fn new() -> Self {
        VariableContext { next_index: 0 }
    }

    pub fn next_variable<T>(&mut self) -> Variable<T> {
        let v = Variable::new(self.next_index);
        self.next_index += 1;
        v
    }
}

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

    pub fn extract(self, binding: Binding) -> T
    where
        T: From<Value>,
    {
        binding.get(self.index).unwrap().into()
    }
}

pub trait Constrain<'a, T> {
    type Constraint: Constraint<'a>;

    fn constrain(&'a self, v: Variable<T>) -> Self::Constraint;
}

impl<T> Variable<T> {
    pub fn of<'a, C>(self, c: &'a C) -> C::Constraint
    where
        C: Constrain<'a, T>,
    {
        c.constrain(self)
    }
}

impl<T> Variable<T> {
    pub fn is(self, constant: T) -> ConstantConstraint<T>
    where
        for<'b> &'b T: Into<Value>,
    {
        ConstantConstraint::new(self, &constant)
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
    fn confirm(&self, variable: VariableId, binding: Binding, proposal: &mut Vec<Value>);
}

pub struct Query<C> {
    constraint: C,
    binding: Binding,
    variables: VariableSet,
    variable_stack: [u8; 256],
    value_stack: [Vec<Value>; 256],
    stack_depth: isize,
}

impl<'a, C: Constraint<'a>> Query<C> {
    pub fn new(constraint: C) -> Self {
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
                            .min_by_key(|&v| self.constraint.estimate(v, self.binding));
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

#[macro_export]
macro_rules! query {
    ($ctx:ident, ($($Var:ident),+), $Constraint:expr) => {
        {
            let mut $ctx = $crate::query::VariableContext::new();
            //let set = $crate::tribleset::patchtribleset::PATCHTribleSet::new();
            $(let $Var = $ctx.next_variable();)*
              $crate::query::Query::new($Constraint).map(
                move |binding| {
                    ($($Var.extract(binding)),+,)
            })
        }
    };
}
pub use query;

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, convert::TryInto};

    use super::*;

    use crate::types::syntactic::shortstring::ShortString;

    #[test]
    fn and_set() {
        let mut books = HashSet::new();
        let mut movies = HashSet::new();

        books.insert(ShortString::new("LOTR".into()).unwrap());
        books.insert(ShortString::new("Dragonrider".into()).unwrap());
        books.insert(ShortString::new("Highlander".into()).unwrap());

        movies.insert(ShortString::new("LOTR".into()).unwrap());
        movies.insert(ShortString::new("Highlander".into()).unwrap());

        let inter: Vec<_> = query!(ctx, (a), and!(a.of(&books), a.of(&movies),)).collect();

        assert_eq!(inter.len(), 2);

        let cross: Vec<_> = query!(ctx, (a, b), and!(a.of(&books), b.of(&movies))).collect();

        assert_eq!(cross.len(), 6);

        let one: Vec<_> = query!(
            ctx,
            (a),
            and!(a.of(&books), a.is("LOTR".try_into().unwrap()))
        )
        .collect();

        assert_eq!(one.len(), 1);

        /*
            query!((a),
                and!(
                    a.of(books),
                    a.of(movies)
                )
            ).collect()

            let inter: Vec<Binding> = Query::new(IntersectionConstraint::new(vec![
            Box::new(SetConstraint::new(a, &books)),
            Box::new(SetConstraint::new(a, &movies)),
            ]))
            .collect();
        */
    }
}
