use std::mem;

use super::*;
use itertools::Itertools;

pub struct UnionConstraint<C> {
    constraints: Vec<C>,
}

impl<'a, C> UnionConstraint<C>
where
    C: Constraint<'a> + 'a,
{
    pub fn new(constraints: Vec<C>) -> Self {
        assert!(constraints
            .iter()
            .map(|c| c.variables())
            .tuple_windows()
            .all(|(a, b)| a == b));
        UnionConstraint { constraints }
    }
}

impl<'a, C> Constraint<'a> for UnionConstraint<C>
where
    C: Constraint<'a> + 'a,
{
    fn variables(&self) -> VariableSet {
        self.constraints[0].variables()
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> Option<usize> {
        self.constraints
            .iter()
            .filter_map(|c| c.estimate(variable, binding))
            .reduce(|acc, e| acc + e)
    }

    fn propose(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let relevant_constraints: Vec<_> = self
            .constraints
            .iter()
            .filter(|c| c.estimate(variable, binding).is_some())
            .collect();

        relevant_constraints
            .iter()
            .for_each(|c| c.propose(variable, binding, proposals));
        proposals.sort();
        proposals.dedup();
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let relevant_constraints: Vec<_> = self
            .constraints
            .iter()
            .filter(|c| c.estimate(variable, binding).is_some())
            .collect();

        proposals.sort();

        let union: Vec<_> = relevant_constraints
            .iter()
            .map(|c| {
                let mut proposals = proposals.clone();
                c.confirm(variable, binding, &mut proposals);
                proposals
            })
            .kmerge()
            .dedup()
            .collect();

        _ = mem::replace(proposals, union);
    }
}

#[macro_export]
macro_rules! or {
    ($($c:expr),+ $(,)?) => (
        $crate::query::unionconstraint::UnionConstraint::new(vec![
            $(Box::new($c) as Box<dyn $crate::query::Constraint>),+
        ])
    )
}

pub use or;
