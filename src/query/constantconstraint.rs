use super::*;

pub struct ConstantConstraint {
    variable: VariableId,
    depth: u8,
    constant: [u8; 32],
}

impl ByteCursor for ConstantConstraint {
    fn peek(&self) -> Peek {
        Peek::Fragment(self.constant[self.depth as usize])
    }

    fn push(&mut self, _byte: u8) {
        self.depth += 1;
    }

    fn pop(&mut self) {
        self.depth -= 1;
    }
}

impl VariableConstraint for ConstantConstraint {
    fn variables(&self) -> VariableSet {
        VariableSet::new_singleton(self.variable)
    }

    fn blocked(&self) -> VariableSet {
        VariableSet::new_empty()
    }

    fn estimate(&self, _variable: VariableId) -> u32 {
        1
    }

    fn explore(&mut self, _variable: VariableId) {}

    fn retreat(&mut self) {}
}
