use super::*;

pub struct IntersectionConstraint {
    constraints: Vec<Box<dyn VariableConstraint>>,
    active_constraints: Vec<usize>,
    variable_stack: Vec<VariableId>,
}

impl ByteCursor for IntersectionConstraint {
    fn peek(&self) -> Peek {
        let mut intersection = ByteBitset::new_empty();
        for constraint_idx in &self.active_constraints {
            intersection = intersection.intersect(match self.constraints[*constraint_idx].peek() {
                Peek::Fragment(byte) => ByteBitset::new_singleton(byte),
                Peek::Branch(children_set) => children_set,
            });
        }
        match intersection.count() {
            1 => Peek::Fragment(
                intersection
                    .find_first_set()
                    .expect("there should be 1 childbit"),
            ),
            _ => Peek::Branch(intersection),
        }
    }

    fn push(&mut self, byte: u8) {
        for constraint in self.constraints.iter_mut() {
            constraint.push(byte);
        }
    }

    fn pop(&mut self) {
        for constraint in self.constraints.iter_mut() {
            constraint.pop();
        }
    }
}

impl VariableConstraint for IntersectionConstraint {
    fn variables(&self) -> VariableSet {
        let mut vars = VariableSet::new_empty();
        for constraint in &self.constraints {
            vars = vars.union(constraint.variables());
        }
        vars
    }

    fn blocked(&self) -> VariableSet {
        let mut blocked = VariableSet::new_empty();
        for constraint in &self.constraints {
            blocked = blocked.union(constraint.blocked());
        }
        blocked
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

    fn explore(&mut self, variable: VariableId) {
        self.variable_stack.push(variable);
        self.active_constraints.clear();
        for (idx, constraint) in self.constraints.iter_mut().enumerate() {
            if constraint.variables().is_set(variable) {
                constraint.push(variable);
                self.active_constraints.push(idx);
            }
        }
    }

    fn retreat(&mut self) {
        for constraint_idx in &self.active_constraints {
            self.constraints[*constraint_idx].pop();
        }
        self.active_constraints.clear();

        self.variable_stack.pop();
        if let Some(current_var) = self.variable_stack.last() {
            for (idx, constraint) in self.constraints.iter().enumerate() {
                if constraint.variables().is_set(*current_var) {
                    self.active_constraints.push(idx);
                }
            }
        }
    }
}
