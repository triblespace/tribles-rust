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
use std::iter::FromIterator;
use std::marker::PhantomData;

pub use constantconstraint::*;
pub use hashsetconstraint::*;
pub use intersectionconstraint::*;
pub use mask::*;
pub use patchconstraint::*;

use crate::{Id, Value, ValueParseError, Valuelike};

use crate::bitset::ByteBitset;

pub trait TriblePattern {
    type PatternConstraint<'a, V>: Constraint<'a>
    where
        V: Valuelike,
        Self: 'a;

    fn pattern<'a, V>(
        &'a self,
        e: Variable<Id>,
        a: Variable<Id>,
        v: Variable<V>,
    ) -> Self::PatternConstraint<'a, V>
    where
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

/// A placeholder for unknowns in a query.
/// Within the query engine each variable is identified by an integer,
/// which can be accessed via the `index` property.
/// Variables also have an associated type which is used to parse the [Value]s
/// found by the query engine.
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

    pub fn extract(self, binding: &Binding) -> Result<T, crate::ValueParseError> {
        T::from_value(binding.get(self.index).unwrap())
    }
}

pub trait ContainsConstraint<'a, T> {
    type Constraint: Constraint<'a>;

    fn has(&'a self, v: Variable<T>) -> Self::Constraint;
}

impl<T> Variable<T> {
    pub fn is(self, constant: T) -> ConstantConstraint<T>
    where
        T: Valuelike,
    {
        ConstantConstraint::new(self, constant)
    }
}

#[derive(Clone, Debug)]
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
    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize;
    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<Value>;
    fn confirm(&self, variable: VariableId, binding: &Binding, proposal: &mut Vec<Value>);
}

pub struct State {
    variable: VariableId,
    values: Vec<Value>,
}
pub struct Query<C, P: Fn(&Binding) -> Result<R, ValueParseError>, R> {
    constraint: C,
    postprocessing: P,
    mode: Search,
    binding: Binding,
    stack: Vec<State>,
    unbound: Vec<VariableId>,
}

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> Result<R, ValueParseError>, R> Query<C, P, R> {
    pub fn new(constraint: C, postprocessing: P) -> Self {
        let variables = constraint.variables();
        Query {
            constraint,
            postprocessing,
            mode: Search::Vertical,
            binding: Default::default(),
            stack: Vec::new(),
            unbound: Vec::from_iter(variables),
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Search {
    Vertical,
    Horizontal,
    Backtrack,
    Done,
}

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> Result<R, ValueParseError>, R> Iterator
    for Query<C, P, R>
{
    // we will be counting with usize
    type Item = Result<R, ValueParseError>;

    // next() is the only required method
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &self.mode {
                Search::Vertical => {
                    self.mode = Search::Horizontal;

                    match self.unbound.len() {
                        0 => {
                            return Some((self.postprocessing)(&self.binding));
                        }
                        1 => {
                            let next_variable = self.unbound.pop().unwrap();
                            self.stack.push(State {
                                variable: next_variable,
                                values: self.constraint.propose(next_variable, &self.binding),
                            })
                        }
                        _ => {
                            let (index, &next_variable) = self
                                .unbound
                                .iter()
                                .enumerate()
                                .min_by_key(|(_, &v)| self.constraint.estimate(v, &self.binding))
                                .unwrap();
                            self.unbound.swap_remove(index);
                            self.stack.push(State {
                                variable: next_variable,
                                values: self.constraint.propose(next_variable, &self.binding),
                            });
                        }
                    }
                }
                Search::Horizontal => {
                    if let Some(state) = self.stack.last_mut() {
                        if let Some(assignment) = state.values.pop() {
                            self.binding.set(state.variable, assignment);
                            self.mode = Search::Vertical;
                        } else {
                            self.mode = Search::Backtrack;
                        }
                    } else {
                        self.mode = Search::Done;
                        return None;
                    }
                }
                Search::Backtrack => {
                    if let Some(state) = self.stack.pop() {
                        self.binding.unset(state.variable);
                        self.unbound.push(state.variable);
                        self.mode = Search::Horizontal;
                    } else {
                        self.mode = Search::Done;
                        return None;
                    }
                }
                Search::Done => {
                    return None;
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

    //use crate::tribleset::patchtribleset::PATCHTribleSet;
    use crate::{types::ShortString, ufoid, Id, TribleSet, NS};

    use super::*;

    NS! {
        pub namespace knights {
            "8143F46E812E88C4544E7094080EC523" as loves: Id;
            "D6E0F2A6E5214E1330565B4D4138E55C" as name: ShortString;
        }
    }

    #[test]
    fn and_set() {
        let mut books = HashSet::new();
        let mut movies = HashSet::new();

        books.insert(ShortString::new("LOTR").unwrap());
        books.insert(ShortString::new("Dragonrider").unwrap());
        books.insert(ShortString::new("Highlander").unwrap());

        movies.insert(ShortString::new("LOTR").unwrap());
        movies.insert(ShortString::new("Highlander").unwrap());

        let inter: Vec<_> = find!(ctx, (a), and!(books.has(a), movies.has(a))).collect();

        assert_eq!(inter.len(), 2);

        let cross: Vec<_> = find!(ctx, (a, b), and!(books.has(a), movies.has(b))).collect();

        assert_eq!(cross.len(), 6);

        let one: Vec<_> = find!(
            ctx,
            (a),
            and!(books.has(a), a.is("LOTR".try_into().unwrap()))
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
        let romeo = ufoid();
        let juliet = ufoid();
        let waromeo = ufoid();
        let mut kb = TribleSet::new();

        kb.union(knights::entity!(juliet,
        {
            name: "Juliet".try_into().unwrap(),
            loves: romeo
        }));

        kb.union(knights::entity!(romeo, {
            name: "Romeo".try_into().unwrap(),
            loves: juliet
        }));
        kb.union(knights::entity!(waromeo, {
            name: "Romeo".try_into().unwrap()
        }));

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
