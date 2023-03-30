use std::rc::Rc;

use super::*;

pub struct ConstantCursor {
    depth: u8,
    constant: [u8; 32],
}
pub struct ConstantConstraint {
    variable: VariableId,
    constant: [u8; 32],
}

impl ByteCursor for ConstantCursor {
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

impl ConstantCursor {
    fn new(constant: [u8; 32]) -> Rc<dyn ByteCursor> {
        Rc::new(Self { depth: 0, constant })
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

    fn explore(&mut self, _variable: VariableId) -> Rc<dyn ByteCursor> {
        ConstantCursor::new(self.constant)
    }

    fn retreat(&mut self, variable: VariableId) {}
}
