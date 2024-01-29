//! Queries allow you to retrieve data by describing the patterns you are looking for.
//! 
//! The query engine provided here has the design goals of extreme simplicity,
//! low and consistent latency, skew resistence, with no tuning required (or possible).
//! To achieve this it implements a novel constraint solving algorithm based on the theory
//! of worst case optimal joins.
//! 
//! New constraints can be implemented via the [Constraint] trait,
//! providing great flexibililty in the way different query operators, 
//! sub-languages, and data-sources can be composed.
//! 
//! 
pub mod constantconstraint;
pub mod hashsetconstraint;
pub mod intersectionconstraint;
pub mod mask;
pub mod patchconstraint;

use std::fmt;
use std::marker::PhantomData;

pub use constantconstraint::*;
pub use hashsetconstraint::*;
pub use intersectionconstraint::*;
pub use mask::*;
pub use patchconstraint::*;

use crate::types::{Value, ValueParseError, Idlike, Valuelike};

use crate::bitset::ByteBitset;

pub trait TriblePattern {
    type PatternConstraint<'a, E, A, V>: Constraint<'a>
    where
        E: Idlike,
        A: Idlike,
        V: Valuelike,
        Self: 'a;

    fn pattern<'a, E, A, V>(
        &'a self,
        e: Variable<E>,
        a: Variable<A>,
        v: Variable<V>,
    ) -> Self::PatternConstraint<'a, E, A, V>
    where
        E: Idlike,
        A: Idlike,
        V: Valuelike;
}

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

    pub fn next_variable<T>(&mut self) -> Variable<T>
    where
        T: Valuelike,
    {
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

impl<T> Variable<T>
where
    T: Valuelike,
{
    pub fn new(index: VariableId) -> Self {
        Variable {
            index,
            typed: PhantomData,
        }
    }

    pub fn extract(self, binding: &Binding) -> Result<T, crate::types::ValueParseError> {
        T::from_value(binding.get(self.index).unwrap())
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
        T: Valuelike,
    {
        ConstantConstraint::new(self, constant)
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
    fn variable(&self, variable: VariableId) -> bool;
    fn estimate(&self, variable: VariableId, binding: Binding) -> usize;
    fn propose(&self, variable: VariableId, binding: Binding) -> Vec<Value>;
    fn confirm(&self, variable: VariableId, binding: Binding, proposal: &mut Vec<Value>);
}

pub struct Query<C, P: Fn(&Binding) -> Result<R, ValueParseError>, R> {
    constraint: C,
    binding: Binding,
    variables: VariableSet,
    variable_stack: [u8; 256],
    value_stack: [Vec<Value>; 256],
    stack_depth: isize,
    postprocessing: P,
}

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> Result<R, ValueParseError>, R> Query<C, P, R> {
    pub fn new(constraint: C, postprocessing: P) -> Self {
        let variables = constraint.variables();
        Query {
            constraint,
            binding: Default::default(),
            variables,
            variable_stack: [0; 256],
            value_stack: std::array::from_fn(|_| vec![]),
            stack_depth: -1,
            postprocessing,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Search {
    Vertical,
    Horizontal,
    Backtrack,
}

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> Result<R, ValueParseError>, R> Iterator
    for Query<C, P, R>
{
    // we will be counting with usize
    type Item = Result<R, ValueParseError>;

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
                        return Some((self.postprocessing)(&self.binding));
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

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> Result<R, ValueParseError>, R> fmt::Debug
    for Query<C, P, R>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Query")
    }
}

#[macro_export]
macro_rules! find {
    ($ctx:ident, ($($Var:ident),+), $Constraint:expr) => {
        {
            let mut $ctx = $crate::query::VariableContext::new();
            $(let $Var = $ctx.next_variable();)*
              $crate::query::Query::new($Constraint,
                move |binding| {
                    Ok(($($Var.extract(binding)?),+,))
            })
        }
    };
}
pub use find;

#[cfg(test)]
mod tests {
    //use fake::faker::name::raw::*;
    //use fake::locales::*;
    //use fake::{Dummy, Fake, Faker};
    use std::{collections::HashSet, convert::TryInto};

    use crate::{patch, NS};
    //use crate::tribleset::patchtribleset::PATCHTribleSet;
    use crate::types::syntactic::shortstring::ShortString;

    use super::*;

    NS! {
        pub namespace knights {
            @ crate::types::syntactic::UFOID;
            loves: "328edd7583de04e2bedd6bd4fd50e651" as crate::types::syntactic::UFOID;
            name: "328147856cc1984f0806dbb824d2b4cb" as crate::types::syntactic::ShortString;
        }
    }

    #[test]
    fn and_set() {
        let mut books = HashSet::new();
        let mut movies = HashSet::new();

        books.insert(ShortString::new("LOTR".into()).unwrap());
        books.insert(ShortString::new("Dragonrider".into()).unwrap());
        books.insert(ShortString::new("Highlander".into()).unwrap());

        movies.insert(ShortString::new("LOTR".into()).unwrap());
        movies.insert(ShortString::new("Highlander".into()).unwrap());

        let inter: Vec<_> = find!(ctx, (a), and!(a.of(&books), a.of(&movies),)).collect();

        assert_eq!(inter.len(), 2);

        let cross: Vec<_> = find!(ctx, (a, b), and!(a.of(&books), b.of(&movies))).collect();

        assert_eq!(cross.len(), 6);

        let one: Vec<_> = find!(
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

    #[test]
    fn pattern() {
        patch::init();

        let kb = knights::entities!((romeo, juliet, waromeo),
        [{juliet @
            name: "Juliet".try_into().unwrap(),
            loves: romeo
        },
        {romeo @
            name: "Romeo".try_into().unwrap(),
            loves: juliet
        },
        {waromeo @
            name: "Romeo".try_into().unwrap()
        }]);

        let r: Vec<_> = find!(
            ctx,
            (romeo, juliet, name),
            knights::pattern!(ctx, kb, [
            {romeo @
                name: ("Romeo".try_into().unwrap()),
             loves: juliet},
            {juliet @
                name: name
            }])
        )
        .collect();

        assert_eq!(1, r.len())
    }
}
