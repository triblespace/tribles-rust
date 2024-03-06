use super::*;

pub struct IntersectionConstraint<'a> {
    constraints: Vec<Box<dyn Constraint<'a> + 'a>>,
}

impl<'a> IntersectionConstraint<'a> {
    pub fn new(constraints: Vec<Box<dyn Constraint<'a> + 'a>>) -> Self {
        IntersectionConstraint { constraints }
    }
}

impl<'a> Constraint<'a> for IntersectionConstraint<'a> {
    fn variables(&self) -> VariableSet {
        self.constraints
            .iter()
            .fold(VariableSet::new_empty(), |vs, c| vs.union(c.variables()))
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.constraints.iter().any(|c| c.variable(variable))
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        self.constraints
            .iter()
            .filter(|c| c.variable(variable))
            .map(|c| c.estimate(variable, binding))
            .min()
            .unwrap()
    }

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<Value> {
        let mut relevant_constraints: Vec<_> = self
            .constraints
            .iter()
            .filter(|c| c.variable(variable))
            .collect();
        relevant_constraints.sort_by_cached_key(|c| c.estimate(variable, binding));

        let mut proposal = relevant_constraints[0].propose(variable, binding);

        relevant_constraints[1..]
            .iter()
            .for_each(|c| c.confirm(variable, binding, &mut proposal));

        proposal
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<Value>) {
        let mut relevant_constraints: Vec<_> = self
            .constraints
            .iter()
            .filter(|c| c.variable(variable))
            .collect();
        relevant_constraints.sort_by_cached_key(|c| c.estimate(variable, binding));

        relevant_constraints
            .iter()
            .for_each(|c| c.confirm(variable, binding, proposals));
    }
}

#[macro_export]
macro_rules! and {
    ($($c:expr),+ $(,)?) => (
        $crate::query::intersectionconstraint::IntersectionConstraint::new(vec![
            $(Box::new($c)),+
        ])
    )
}

pub use and;
