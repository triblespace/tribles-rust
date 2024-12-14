//! Queries allow you to retrieve data by describing the patterns you are looking for.
//!
//! The query engine provided here has the design goals of extreme simplicity,
//! low, consistent, and predictable latency, skew resistence, with no tuning required (or possible).
//!
//! New constraints can be implemented via the [Constraint] trait,
//! providing great flexibililty in the way different query operators,
//! sub-languages, and data-sources can be composed.
//!
//! # Queries as Schemas
//!
//! You might already have noticed that trible.space does not have any concept
//! of an ontology or schema specification beyond the association of attributes
//! with [ValueSchema] and [crate::prelude::BlobSchema]. This is deliberate, as one of our
//! lessons learned from the semantic web was that it is too loose in the typing
//! of individual values, but too strict and computationally infeasible in the
//! description of larger structures. Any system that deals with real-world data
//! grounded in reality will need to robustly handle cases of missing,
//! duplicate, or additonal fields, which is fundamentally in conflict with
//! strong constraints like classes.
//!
//! Our approach is to be sympathetic to the edge case and has the system deal
//! only with the data that it declares capable of handling. These "application
//! specific schema declarations" are exactly the shapes and constraints
//! described by our queries[^1], with data not conforming to these
//! queries/schemas simply being ignored by definition (as a query only returns
//! the data conforming to it's constraints).[^2]
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
//! [^1]: Note that this query-schema isomorphism isn't nessecarily true in all
//! databases or query languages, e.g. it does not hold for SQL.
//! [^2]: In RDF terminology:
//! We challenge the classical A-Box & T-Box dichotomy. Replacing the T-Box with
//! a "Q-Box", which instead of being prescriptive and closed, is descriptive
//! and open. This Q-Box naturally evolves with new and changing requirements,
//! contexts, and applications.
//!
pub mod constantconstraint;
pub mod hashmapconstraint;
pub mod hashsetconstraint;
pub mod intersectionconstraint;
pub mod mask;
pub mod patchconstraint;
pub mod unionconstraint;
mod variableset;

use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;

use arrayvec::ArrayVec;
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

    pub fn extract(self, binding: &Binding) -> &Value<T> {
        Value::transmute_raw(binding.get(self.index).unwrap())
    }
}

pub trait ContainsConstraint<'a, T: ValueSchema> {
    type Constraint: Constraint<'a>;

    fn has(self, v: Variable<T>) -> Self::Constraint;
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
    fn propose(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>);
    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>);
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

    fn propose(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let inner: &T = self;
        inner.propose(variable, binding, proposals)
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let inner: &T = self;
        inner.confirm(variable, binding, proposals)
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

    fn propose(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let inner: &T = self;
        inner.propose(variable, binding, proposals)
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposal: &mut Vec<RawValue>) {
        let inner: &T = self;
        inner.confirm(variable, binding, proposal)
    }
}

pub struct Query<C, P: Fn(&Binding) -> R, R> {
    constraint: C,
    postprocessing: P,
    mode: Search,
    binding: Binding,
    stack: ArrayVec<VariableId, 128>,
    unbound: ArrayVec<VariableId, 128>,
    values: [Vec<RawValue>; 128],
}

impl<'a, C: Constraint<'a>, P: Fn(&Binding) -> R, R> Query<C, P, R> {
    pub fn new(constraint: C, postprocessing: P) -> Self {
        let variables = constraint.variables();
        Query {
            constraint,
            postprocessing,
            mode: Search::Vertical,
            binding: Default::default(),
            stack: ArrayVec::new(),
            unbound: ArrayVec::from_iter(variables),
            values: [const { vec![] }; 128],
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
                            self.stack.push(next_variable);
                            self.constraint.propose(
                                next_variable,
                                &self.binding,
                                &mut self.values[next_variable as usize],
                            );
                        }
                        _ => {
                            let (index, &next_variable) = self
                                .unbound
                                .iter()
                                .enumerate()
                                .min_by_key(|(_, &v)| self.constraint.estimate(v, &self.binding))
                                .unwrap();
                            self.unbound.swap_remove(index);
                            self.stack.push(next_variable);
                            self.constraint.propose(
                                next_variable,
                                &self.binding,
                                &mut self.values[next_variable as usize],
                            );
                        }
                    }
                }
                Search::Horizontal => {
                    if let Some(&variable) = self.stack.last() {
                        if let Some(assignment) = self.values[variable as usize].pop() {
                            self.binding.set(variable, &assignment);
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
                    if let Some(variable) = self.stack.pop() {
                        self.binding.unset(variable);
                        self.values[variable as usize].clear();
                        self.unbound.push(variable);
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
    (($($Var:tt$(:$Ty:ty)?),+), $Constraint:expr) => {
        {
            let mut ctx = $crate::query::VariableContext::new();

            macro_rules! __local_find_context {
                () => {&mut ctx}
            }

            $(let $Var = ctx.next_variable();)*
              $crate::query::Query::new($Constraint,
                move |binding| {
                    $(let $Var$(:$Ty)? = $crate::value::FromValue::from_value($Var.extract(binding));)+
                    ($($Var),+,)
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

    use crate::tests::literature;

    use fake::faker::lorem::en::{Sentence, Word};
    use fake::faker::name::raw::*;
    use fake::locales::*;
    use fake::Fake;

    use std::collections::HashSet;

    use super::*;

    NS! {
        pub namespace knights5 {
            "8143F46E812E88C4544E7094080EC523" as loves: GenId;
            "D6E0F2A6E5214E1330565B4D4138E55C" as name: ShortString;
        }
    }

    #[test]
    fn and_set() {
        let mut books = HashSet::<String>::new();
        let mut movies = HashSet::<Value<ShortString>>::new();

        books.insert("LOTR".to_string());
        books.insert("Dragonrider".to_string());
        books.insert("Highlander".to_string());

        movies.insert("LOTR".to_value());
        movies.insert("Highlander".to_value());

        let inter: Vec<_> =
            find!((a: Value<ShortString>), and!(books.has(a), movies.has(a))).collect();

        assert_eq!(inter.len(), 2);

        let cross: Vec<_> =
            find!((a: Value<ShortString>, b: Value<ShortString>), and!(books.has(a), movies.has(b))).collect();

        assert_eq!(cross.len(), 6);

        let one: Vec<_> = find!((a: Value<ShortString>),
            and!(books.has(a), a.is("LOTR".try_to_value().unwrap())) //TODO
        )
        .collect();

        assert_eq!(one.len(), 1);
    }

    #[test]
    fn pattern() {
        let mut kb = TribleSet::new();
        (0..1000000).for_each(|_| {
            let author = fucid();
            let book = fucid();
            kb += literature::entity!(&author, {
                firstname: FirstName(EN).fake::<String>(),
                lastname: LastName(EN).fake::<String>(),
            });
            kb += literature::entity!(&book, {
                author: &author,
                title: Word().fake::<String>(),
                quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
            });
        });

        let author = fucid();
        let book = fucid();
        kb += literature::entity!(&author, {
            firstname: "Frank",
            lastname: "Herbert",
        });
        kb += literature::entity!(&book, {
            author: &author,
            title: "Dune",
            quote: "I must not fear. Fear is the \
                    mind-killer. Fear is the little-death that brings total \
                    obliteration. I will face my fear. I will permit it to \
                    pass over me and through me. And when it has gone past I \
                    will turn the inner eye to see its path. Where the fear \
                    has gone there will be nothing. Only I will remain.".to_blob().as_handle()
        });

        (0..1000).for_each(|_| {
            let author = fucid();
            let book = fucid();
            kb += literature::entity!(&author, {
                firstname: "Fake",
                lastname: "Herbert",
            });
            kb += literature::entity!(&book, {
                author: &author,
                title: Word().fake::<String>(),
                quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
            });
        });

        let r: Vec<_> = find!(
        (author: Value<_>, book: Value<_>, title: Value<_>, quote: Value<_>),
        literature::pattern!(&kb, [
        {author @
            firstname: ("Frank"),
            lastname: ("Herbert")},
        {book @
          author: author,
          title: title,
          quote: quote
        }]))
        .collect();

        assert_eq!(1, r.len())
    }

    #[test]
    fn constant() {
        let q: Query<IntersectionConstraint<_>, _, _> = find! {
            (string: Value<_>, number: Value<_>),
            and!(
                string.is(ShortString::to_value("Hello World!")),
                number.is(I256BE::to_value(42))
            )
        };
        let r: Vec<_> = q.collect();

        assert_eq!(1, r.len())
    }
}
