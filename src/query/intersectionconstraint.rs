use std::rc::Rc;

use super::*;


pub struct IntersectionConstraintIter<'a> {
    proposer: Box<dyn Iterator<Item = Value> + 'a>,
    confimers: Vec<&'a Box<dyn Constraint<'a> + 'a>>
}

pub struct IntersectionConstraint<'a> {
    constraints: Vec<Box<dyn Constraint<'a> + 'a>>
}

impl<'a> Constraint<'a> for IntersectionConstraint<'a> {
    fn variables(&self) -> VariableSet {
        self.constraints
            .iter()
            .fold(VariableSet::new_empty(), |vs, c| vs.union(c.variables()))
    }

    fn estimate(&self, variable: VariableId) -> usize {
        self.constraints
            .iter()
            .filter(|c| c.variables().is_set(variable))
            .map(|c| c.estimate(variable))
            .min()
            .unwrap()
    }

    fn propose<'b>(&'b self, variable: VariableId, binding: Binding) -> Box<dyn Iterator<Item = Value> + 'b>
    where 'a: 'b
    {
        let mut relevant_constraints: Vec<_> = self
            .constraints
            .iter()
            .filter(|c| c.variables().is_set(variable))
            .collect();
        relevant_constraints.sort_by_key(|c| c.estimate(variable));

        Box::new(
            relevant_constraints[0]
                .propose(variable, binding)
                .filter(move |v| {
                    relevant_constraints[1..]
                        .iter()
                        .all(|c| c.confirm(variable, *v, binding))
                }),
        )
    }

    fn confirm(&self, variable: VariableId, value: Value, binding: Binding) -> bool {
        self.constraints
            .iter()
            .all(|c| c.confirm(variable, value, binding))
    }
}
