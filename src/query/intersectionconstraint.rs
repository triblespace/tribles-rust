use im::Vector;

use super::*;

pub struct IntersectionConstraint {
    constraints: Vec<Box<dyn Constraint>>,
}

impl Constraint for IntersectionConstraint {
    fn variables(&self) -> VariableSet {
        self.constraints.iter().fold(VariableSet::new_empty(), 
            |vs, c| vs.union(c.variables()))
    }

    fn estimate(&self, variable: VariableId) -> u32 {
        let mut min = u32::MAX;
        for constraint in &self.constraints {
            if constraint.variables().is_set(variable) {
                min = std::cmp::min(min, constraint.estimate(variable));
            }
        }
        min
    }

    fn propose(&self, variable: VariableId, binding: Binding) -> Box<dyn Iterator<Item = Value>> {
        let relevant_constraints: Vec<_> = self.constraints.iter().filter(|c| c.variables().is_set(variable)).collect();
        relevant_constraints.sort_by_key(|c| c.estimate(variable));
        let proposer = relevant_constraints[0];
        let confirms = relevant_constraints[1..];

        proposer.propose(variable, binding).filter(|v| confirms.iter().all(|c| c.confirm(variable, v, binding)))
    }

    fn confirm(&self, variable: VariableId, value: Value, binding: Binding) -> bool {
        self.constraints.iter().all(|c| c.confirm(variable, value, binding))
    }
}
