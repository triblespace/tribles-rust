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

    fn estimate(&self, variable: VariableId, binding: Binding) -> usize {
        self.constraints
            .iter()
            .filter(|c| c.variables().is_set(variable))
            .map(|c| c.estimate(variable, binding))
            .min()
            .unwrap()
    }

    fn propose(&self, variable: VariableId, binding: Binding) -> Vec<Value> {
        let mut relevant_constraints: Vec<_> = self
            .constraints
            .iter()
            .filter(|c| c.variables().is_set(variable))
            .collect();
        relevant_constraints.sort_by_key(|c| c.estimate(variable, binding));

        relevant_constraints[0]
            .propose(variable, binding)
            .into_iter()
            .filter(|v| {
                relevant_constraints[1..]
                    .iter()
                    .all(|c| c.confirm(variable, *v, binding))
            })
            .collect()
    }

    fn confirm(&self, variable: VariableId, value: Value, binding: Binding) -> bool {
        self.constraints
            .iter()
            .all(|c| c.confirm(variable, value, binding))
    }
}

macro_rules! and {
    ($($c:expr),+ $(,)?) => (
        IntersectionConstraint::new(vec![
            $(Box::new($c)),+
        ])
    )
}

pub(crate) use and;
