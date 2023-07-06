use super::*;
/*
pub trait ByteCursor {
    const LEN: usize;

    fn peek(&self) -> Peek;

    fn push(&mut self, byte: u8);

    fn pop(&mut self);
}

#[derive(Debug, Copy, Clone)]
enum ExplorationMode {
    Path,
    Branch,
    Backtrack,
}

pub struct CursorIterator<Cursor: ByteCursor> {
    mode: ExplorationMode,
    depth: usize,
    key: [u8; Cursor::LEN],
    branch_points: ByteBitset,
    branch_state: [ByteBitset; Cursor::LEN],
    cursor: Cursor,
}

impl<Cursor: ByteCursor> CursorIterator<Cursor> {
    pub fn new(cursor: Cursor) -> Self {
        Self {
            mode: ExplorationMode::Path,
            depth: 0,
            key: [0; Cursor::LEN],
            branch_points: ByteBitset::new_empty(),
            branch_state: [ByteBitset::new_empty(); Cursor::LEN],
            cursor,
        }
    }
}

impl<Cursor: ByteCursor> Iterator for CursorIterator<Cursor> {
    type Item = [u8; Cursor::LEN];

    fn next(&mut self) -> Option<Self::Item> {
        'search: loop {
            match self.mode {
                ExplorationMode::Path => loop {
                    match self.cursor.peek() {
                        Peek::Fragment(key_fragment) => {
                            self.key[self.depth] = key_fragment;
                            if self.depth == Cursor::LEN - 1 {
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
                        if self.depth == Cursor::LEN - 1 {
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

impl<T: ByteCursor, K> IntoIterator for T
where
    K: KeyProperties<{T::LEN}>,
    [Head<{T::LEN}, K>; T::LEN]: Sized,
{
    type Item = [u8; T::LEN];
    type IntoIter = CursorIterator<Self>;

    fn into_iter(self) -> Self::IntoIter {
        CursorIterator::new(self)
    }
}
*/