use crate::{
    query::{Constraint, Variable},
    types::{Idlike, Valuelike},
};

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
