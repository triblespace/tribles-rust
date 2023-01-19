use crate::bitset::ByteBitset;

enum ExplorationMode { path, branch, backtrack };

pub struct CursorIterator<CURSOR: ByteCursor, const MAX_DEPTH: usize> {
    mode: ExplorationMode,
    depth: usize = 0,
    key: [u8; MAX_DEPTH],
    branch_points: ByteBitset,
    branch_state: [ByteBitset; MAX_DEPTH],
    cursor: cursor_type,
}

impl<CURSOR: ByteCursor, const MAX_DEPTH: usize> CursorIterator<CURSOR, MAX_DEPTH> {
    pub fn new(cursor: CURSOR) Self {
        Self {
            mode: ExplorationMode,
            depth: 0;
            key: [0; MAX_DEPTH],
            branch_points: ByteBitset::new_empty(),
            branch_state: [ByteBitset::new_empty(); MAX_DEPTH],
            cursor
        }
    }
}
impl<CURSOR: ByteCursor, const MAX_DEPTH: usize> Iterator for CursorIterator<CURSOR, MAX_DEPTH> {
    type Item = [u8; MAX_DEPTH];

    pub fn next(self: &mut Self) Option<Self::Item> {
        search: loop {
            match (self.mode) {
                ExplorationMode::path => {
                    while self.depth < MAX_DEPTH {
                        if let Some(key_fragment) = self.cursor.peek() {
                            self.key[self.depth] = key_fragment;
                            self.cursor.push(key_fragment);
                            self.depth += 1;
                        } else {
                            self.cursor.propose(&mut self.branch_state[self.depth]);
                            self.branch_points.set(self.depth);
                            self.mode = ExplorationMode::branch;
                            continue 'search;
                        }
                    }
                    self.mode = ExplorationMode::backtrack;
                    return Some(self.key);
                },
                ExplorationMode::branch => {
                    if let Some(key_fragment) = self.branch_state[self.depth].drainNextAscending() {
                        self.key[self.depth] = key_fragment;
                        self.cursor.push(key_fragment);
                        self.depth += 1;
                        self.mode = ExplorationMode::path;
                    } else {
                        self.branch_points.unset(self.depth);
                        self.mode = ExplorationMode::backtrack;
                    }
                },
                ExplorationMode::backtrack => {
                    if let Some(parent_depth) = self.branch_points.findLastSet() {
                        while parent_depth < self.depth {
                            self.cursor.pop()
                            self.depth -= 1;
                        }
                        self.mode = ExplorationMode::branch;
                    } else {
                        return None;
                    }
                }
            }
        }
    }
}
