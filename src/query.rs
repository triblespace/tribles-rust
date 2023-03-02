mod constantconstraint;
mod intersectionconstraint;

use constantconstraint::*;
use intersectionconstraint::*;

use crate::bitset::ByteBitset;

pub type VariableId = u8;
pub type VariableSet = ByteBitset;

pub enum Peek {
    Fragment(u8),
    Branch(ByteBitset),
}

pub trait ByteCursor {
    fn peek(&self) -> Peek;

    fn push(&mut self, byte: u8);

    fn pop(&mut self);
}

pub trait VariableConstraint: ByteCursor {
    fn variables(&self) -> VariableSet;

    fn blocked(&self) -> VariableSet;

    fn estimate(&self, variable: VariableId) -> u32;

    fn explore(&mut self, variable: VariableId);

    fn retreat(&mut self);
}

#[derive(Debug, Copy, Clone)]
enum ExplorationMode {
    Path,
    Branch,
    Backtrack,
}

pub struct CursorIterator<CURSOR: ByteCursor, const MAX_DEPTH: usize> {
    mode: ExplorationMode,
    depth: usize,
    key: [u8; MAX_DEPTH],
    branch_points: ByteBitset,
    branch_state: [ByteBitset; MAX_DEPTH],
    cursor: CURSOR,
}

impl<CURSOR: ByteCursor, const MAX_DEPTH: usize> CursorIterator<CURSOR, MAX_DEPTH> {
    pub fn new(cursor: CURSOR) -> Self {
        Self {
            mode: ExplorationMode::Path,
            depth: 0,
            key: [0; MAX_DEPTH],
            branch_points: ByteBitset::new_empty(),
            branch_state: [ByteBitset::new_empty(); MAX_DEPTH],
            cursor,
        }
    }
}
impl<CURSOR: ByteCursor, const MAX_DEPTH: usize> Iterator for CursorIterator<CURSOR, MAX_DEPTH> {
    type Item = [u8; MAX_DEPTH];

    fn next(&mut self) -> Option<Self::Item> {
        'search: loop {
            match self.mode {
                ExplorationMode::Path => loop {
                    match self.cursor.peek() {
                        Peek::Fragment(key_fragment) => {
                            self.key[self.depth] = key_fragment;
                            if self.depth == MAX_DEPTH - 1 {
                                self.mode = ExplorationMode::Backtrack;
                                return Some(self.key);
                            } else {
                                self.cursor.push(key_fragment);
                                self.depth += 1;
                            }
                        }
                        Peek::Branch(options) => {
                            self.branch_state[self.depth] = options;
                            self.branch_points.set(self.depth as u8);
                            self.mode = ExplorationMode::Branch;
                            continue 'search;
                        }
                    }
                },
                ExplorationMode::Branch => {
                    if let Some(key_fragment) = self.branch_state[self.depth].drain_next_ascending()
                    {
                        self.key[self.depth] = key_fragment;
                        if self.depth == MAX_DEPTH - 1 {
                            return Some(self.key);
                        } else {
                            self.cursor.push(key_fragment);
                            self.depth += 1;
                            self.mode = ExplorationMode::Path;
                        }
                    } else {
                        self.branch_points.unset(self.depth as u8);
                        self.mode = ExplorationMode::Backtrack;
                    }
                }
                ExplorationMode::Backtrack => {
                    if let Some(parent_depth) = self.branch_points.find_last_set() {
                        while (parent_depth as usize) < self.depth {
                            self.cursor.pop();
                            self.depth -= 1;
                        }
                        self.mode = ExplorationMode::Branch;
                    } else {
                        return None;
                    }
                }
            }
        }
    }
}
