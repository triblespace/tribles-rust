pub mod hashtribleset;
pub mod pacttribleset;

use crate::{
    namespace::{Id, Value},
    query::{Constraint, Variable},
};

pub trait TribleSet {
    type PatternConstraint<'a, E, A, V>: Constraint<'a>
    where
        E: From<Id>,
        A: From<Id>,
        V: From<Value>,
        for<'b> &'b E: Into<Id>,
        for<'b> &'b A: Into<Id>,
        for<'b> &'b V: Into<Value>,
        Self: 'a;

    fn pattern<'a, E, A, V>(
        &'a self,
        e: Variable<E>,
        a: Variable<A>,
        v: Variable<V>,
    ) -> Self::PatternConstraint<'a, E, A, V>
    where
        E: From<Id>,
        A: From<Id>,
        V: From<Value>,
        for<'b> &'b E: Into<Id>,
        for<'b> &'b A: Into<Id>,
        for<'b> &'b V: Into<Value>;
}
