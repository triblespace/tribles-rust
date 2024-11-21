use super::*;

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

    fn variable(&self, variable: VariableId) -> bool {
        self.constraints[0].variable(variable)
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        self.constraints
            .iter()
            .filter(|c| c.variable(variable))
            .map(|c| c.estimate(variable, binding))
            .sum()
    }

    fn propose(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<RawValue>) {
        let relevant_constraints: Vec<_> = self
            .constraints
            .iter()
            .filter(|c| c.variable(variable))
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
            .filter(|c| c.variable(variable))
            .collect();

        relevant_constraints
            .iter()
            .for_each(|c| c.confirm(variable, binding, proposals));
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

use itertools::Itertools;
pub use or;
