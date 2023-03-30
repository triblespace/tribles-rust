use std::rc::Rc;

use super::*;

pub struct IntersectionCursor<const LEN: usize> {
    cursors: [Option<Rc<dyn ByteCursor>>; LEN],
}

impl<const LEN: usize> IntersectionCursor<LEN> {
    fn new(cursors: [Option<Rc<dyn ByteCursor>>; LEN]) -> Rc<dyn ByteCursor> {
        Rc::new(Self { cursors })
    }
}

pub struct IntersectionConstraint<const LEN: usize> {
    constraints: [Rc<dyn VariableConstraint>; LEN],
}

impl<const LEN: usize> ByteCursor for IntersectionCursor<LEN> {
    fn peek(&self) -> Peek {
        let mut intersection = ByteBitset::new_empty();
        for cursor in &self.cursors {
            if let Some(cursor) = cursor {
                intersection = intersection.intersect(match cursor.peek() {
                    Peek::Fragment(byte) => ByteBitset::new_singleton(byte),
                    Peek::Branch(children_set) => children_set,
                });
            }
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
        for cursor in self.cursors.iter_mut() {
            if let Some(cursor) = cursor {
                cursor.push(byte);
            }
        }
    }

    fn pop(&mut self) {
        for cursor in self.cursors.iter_mut() {
            if let Some(cursor) = cursor {
                cursor.pop();
            }
        }
    }
}

const INIT: Option<Rc<dyn ByteCursor>> = None;

impl<const LEN: usize> VariableConstraint for IntersectionConstraint<LEN> {
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

    fn explore(&mut self, variable: VariableId) -> Rc<dyn ByteCursor> {
        let mut cursors = [INIT; LEN];
        for (idx, constraint) in self.constraints.iter_mut().enumerate() {
            if constraint.variables().is_set(variable) {
                cursors[idx] = Some(constraint.explore(variable));
            }
        }
        IntersectionCursor::new(cursors)
    }

    fn retreat(&mut self, variable: VariableId) {
        for constraint in self.constraints.iter_mut() {
            if constraint.variables().is_set(variable) {
                constraint.retreat(variable);
            }
        }
    }
}
