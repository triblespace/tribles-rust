//! Queries allow you to retrieve data by describing the patterns you are looking for.
//!
//! The query engine provided here has the design goals of extreme simplicity,
//! low, consistent, and predictable latency, skew resistence, with no tuning required (or possible).
//!
//! New constraints can be implemented via the [Constraint] trait,
//! providing great flexibililty in the way different query operators,
//! sub-languages, and data-sources can be composed.
//!
//! # The Atreides Family of Worstcase Optimal Join Algorithms
//!
//! The heart of the system is a constraint solving approach based on the theory
//! of worst case optimal joins, specifically a family of novel join algorithms
//! we dubbed the "Atreides-Family".
//!
//! The insigt being that we can use size estimations normally used by the query optimzer
//! to directly guide the join algorithm to retrieve bounds which normally require
//! sorted indexes for the random-access case.
//!
//! As this moves a lot of the execution cost on cardinality estimation we also
//! developed novel datastructures to efficiently maintain these estimates in O(1).
//!
//! We focus on three specific instantiations of the "Atreides-Family",
//! which differ in the quality of the cardiality estimation provided, i.e.
//! the clarity that the algorithm has when looking into the future.
//!
//! Given a _partial_ Binding.
//!
//! - *Jessica's Join* - The smallest number of rows matching the variable.
//! - *Paul's Join* - The smallest number of distinct values from one column matching the variable.
//! - *Leto's Join* - The true number of values matching the variable (e.g. after intersection).
//!
//!
pub mod constantconstraint;
pub mod hashmapconstraint;
pub mod hashsetconstraint;
pub mod intersectionconstraint;
pub mod mask;
pub mod patchconstraint;
mod variableset;

use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;

use constantconstraint::*;
use mask::*;

use crate::value::{schemas::genid::GenId, RawValue, Value, ValueSchema};

pub use variableset::VariableSet;

pub trait TriblePattern {
    type PatternConstraint<'a>: Constraint<'a>
    where
        Self: 'a;

    fn pattern<'a, V: ValueSchema>(
        &'a self,
        e: Variable<GenId>,
        a: Variable<GenId>,
        v: Variable<V>,
    ) -> Self::PatternConstraint<'a>;
}

pub type VariableId = u8;

#[derive(Debug)]
pub struct VariableContext {
    pub next_index: VariableId,
}

impl VariableContext {
    pub fn new() -> Self {
        VariableContext { next_index: 0 }
    }

    pub fn next_variable<T: ValueSchema>(&mut self) -> Variable<T> {
        assert!(
            self.next_index < 128,
            "currently queries support at most 128 variables"
        );
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
pub struct Variable<T: ValueSchema> {
    pub index: VariableId,
    typed: PhantomData<T>,
}

impl<T: ValueSchema> Copy for Variable<T> {}

impl<T: ValueSchema> Clone for Variable<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ValueSchema> Variable<T> {
    pub fn new(index: VariableId) -> Self {
        Variable {
            index,
            typed: PhantomData,
        }
    }

    pub fn extract(self, binding: &Binding) -> Value<T> {
        Value::new(*binding.get(self.index).unwrap())
    }
}

pub trait ContainsConstraint<'a, T: ValueSchema> {
    type Constraint: Constraint<'a>;

    fn has(&'a self, v: Variable<T>) -> Self::Constraint;
}

impl<T: ValueSchema> Variable<T> {
    pub fn is(self, constant: Value<T>) -> ConstantConstraint {
        ConstantConstraint::new(self, constant)
    }
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub bound: VariableSet,
    values: [RawValue; 128],
}

impl Binding {
    pub fn set(&mut self, variable: VariableId, value: &RawValue) {
        self.values[variable as usize] = *value;
        self.bound.set(variable);
    }

    pub fn unset(&mut self, variable: VariableId) {
        self.bound.unset(variable);
    }

    pub fn get(&self, variable: VariableId) -> Option<&RawValue> {
        //TODO check if we should make this a ref
        if self.bound.is_set(variable) {
            Some(&self.values[variable as usize])
        } else {
            None
        }
    }
}

impl Default for Binding {
    fn default() -> Self {
        Self {
            bound: VariableSet::new_empty(),
            values: [[0; 32]; 128],
        }
    }
}

pub trait Constraint<'a> {
    fn variables(&self) -> VariableSet;
    fn variable(&self, variable: VariableId) -> bool;
    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize;
    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<RawValue>;
    fn confirm(&self, variable: VariableId, binding: &Binding, proposal: &mut Vec<RawValue>);
}

impl<'a, T: Constraint<'a> + ?Sized> Constraint<'a> for Box<T> {
    fn variables(&self) -> VariableSet {
        let inner: &T = self;
        inner.variables()
    }

    fn variable(&self, variable: VariableId) -> bool {
        let inner: &T = self;
        inner.variable(variable)
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        let inner: &T = self;
        inner.estimate(variable, binding)
    }

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<RawValue> {
        let inner: &T = self;
        inner.propose(variable, binding)
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposal: &mut Vec<RawValue>) {
        let inner: &T = self;
        inner.confirm(variable, binding, proposal)
    }
}

impl<'a, T: Constraint<'a> + ?Sized> Constraint<'static> for std::sync::Arc<T> {
    fn variables(&self) -> VariableSet {
        let inner: &T = self;
        inner.variables()
    }

    fn variable(&self, variable: VariableId) -> bool {
        let inner: &T = self;
        inner.variable(variable)
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        let inner: &T = self;
        inner.estimate(variable, binding)
    }

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<RawValue> {
        let inner: &T = self;
        inner.propose(variable, binding)
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposal: &mut Vec<RawValue>) {
        let inner: &T = self;
        inner.confirm(variable, binding, proposal)
    }
}

pub struct State {
    variable: VariableId,
    values: Vec<RawValue>,
}
pub struct Query<C, P: Fn(&Binding) -> R, R> {
    constraint: C,
    postprocessing: P,
    mode: Search,
    binding: Binding,
    stack: Vec<State>,
    unbound: Vec<VariableId>,
}

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> R, R> Query<C, P, R> {
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

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> R, R> Iterator for Query<C, P, R> {
    type Item = R;

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
                            self.binding.set(state.variable, &assignment);
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

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> R, R> fmt::Debug for Query<C, P, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Query") //TODO
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
                    ($($Var.extract(binding)),+,)
            })
        }
    };
}
pub use find;

#[cfg(test)]
mod tests {
    use valueschemas::ShortString;

    use crate::prelude::valueschemas::*;
    use crate::prelude::*;

    use std::collections::HashSet;

    use super::*;

    NS! {
        pub namespace knights {
            "8143F46E812E88C4544E7094080EC523" as loves: GenId;
            "D6E0F2A6E5214E1330565B4D4138E55C" as name: ShortString;
        }
    }

    #[test]
    fn and_set() {
        let mut books = HashSet::<Value<ShortString>>::new();
        let mut movies = HashSet::<Value<ShortString>>::new();

        books.insert("LOTR".try_pack().unwrap());
        books.insert("Dragonrider".try_pack().unwrap());
        books.insert("Highlander".try_pack().unwrap());

        movies.insert("LOTR".try_pack().unwrap());
        movies.insert("Highlander".try_pack().unwrap());

        let inter: Vec<_> = find!(ctx, (a), and!(books.has(a), movies.has(a))).collect();

        assert_eq!(inter.len(), 2);

        let cross: Vec<_> = find!(ctx, (a, b), and!(books.has(a), movies.has(b))).collect();

        assert_eq!(cross.len(), 6);

        let one: Vec<_> = find!(
            ctx,
            (a),
            and!(books.has(a), a.is("LOTR".try_pack().unwrap())) //TODO
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
            name: "Juliet".try_pack().unwrap(),
            loves: romeo.into()
        }));

        kb.union(knights::entity!(romeo, {
            name: "Romeo".try_pack().unwrap(),
            loves: juliet.into()
        }));
        kb.union(knights::entity!(waromeo, {
            name: "Romeo".try_pack().unwrap()
        }));

        let q: Query<IntersectionConstraint<Box<dyn Constraint<'static>>>, _, _> = find!(
            ctx,
            (romeo, juliet, name),
            knights::pattern!(ctx, &kb, [
            {romeo @
                name: ("Romeo".try_pack().unwrap()),
             loves: juliet},
            {juliet @
                name: name
            }])
        );

        let r: Vec<_> = q.collect();

        assert_eq!(1, r.len())
    }

    #[test]
    fn constant() {
        let q: Query<IntersectionConstraint<_>, _, _> = find!(
            ctx,
            (string, number),
            and!(
                string.is(ShortString::try_pack("Hello World!").unwrap()),
                number.is(I256BE::pack(&42))
            )
        );
        let r: Vec<_> = q.collect();

        assert_eq!(1, r.len())
    }
}
