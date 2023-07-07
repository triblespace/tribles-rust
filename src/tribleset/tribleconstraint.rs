/*
use crate::pact::PaddedCursor;
use crate::query::*;
use crate::trible::{AEVOrder, AVEOrder, EAVOrder};

use super::TribleSet;

pub enum Stack {
    Empty,
    E,
    A,
    V,
    EA,
    EV,
    AE,
    AV,
    VE,
    VA,
    EAV,
    EVA,
    AEV,
    AVE,
    VEA,
    VAE,
}

pub struct TribleConstraint {
    state: Stack,
    e_var: VariableId,
    a_var: VariableId,
    v_var: VariableId,
    eav_cursor: PaddedCursor<64, EAVOrder>,
    aev_cursor: PaddedCursor<64, AEVOrder>,
    ave_cursor: PaddedCursor<64, AVEOrder>,
}

impl TribleConstraint {
    fn new(set: TribleSet, e: VariableId, a: VariableId, v: VariableId) -> TribleConstraint {
        if e == a || e == v || a == v {
            panic!(
              "Triple variables must be uniqe. Use explicit equality when inner constraints are required.",
            );
        }

        TribleConstraint {
            state: Stack::Empty,
            e_var: e,
            a_var: a,
            v_var: v,
            eav_cursor: set.eav.padded_cursor(),
            aev_cursor: set.aev.padded_cursor(),
            ave_cursor: set.ave.padded_cursor(),
        }
    }
}

impl ByteCursor for TribleConstraint {
    fn peek(&self) -> Peek {
        match self.state {
            Stack::E | Stack::EA | Stack::EAV => self.eav_cursor.peek(),
            Stack::A | Stack::AE | Stack::AEV => self.aev_cursor.peek(),
            Stack::AV | Stack::AVE => self.ave_cursor.peek(),
            _ => panic!("unreachable"),
        }
    }

    fn push(&mut self, byte: u8) {
        match self.state {
            Stack::A => {
                self.aev_cursor.push(byte);
                self.ave_cursor.push(byte)
            }
            Stack::E | Stack::EA | Stack::EAV => self.eav_cursor.push(byte),
            Stack::AE | Stack::AEV => self.aev_cursor.push(byte),
            Stack::AV | Stack::AVE => self.ave_cursor.push(byte),
            _ => panic!("unreachable"),
        }
    }

    fn pop(&mut self) {
        match self.state {
            Stack::A => {
                self.aev_cursor.pop();
                self.ave_cursor.pop()
            }
            Stack::E | Stack::EA | Stack::EAV => self.eav_cursor.pop(),
            Stack::AE | Stack::AEV => self.aev_cursor.pop(),
            Stack::AV | Stack::AVE => self.ave_cursor.pop(),
            _ => panic!("unreachable"),
        }
    }
}

impl VariableConstraint for TribleConstraint {
    fn variables(&self) -> VariableSet {
        let mut var_set = VariableSet::new_empty();
        var_set.set(self.e_var);
        var_set.set(self.a_var);
        var_set.set(self.v_var);
        return var_set;
    }

    fn blocked(&self) -> VariableSet {
        match self.state {
            Stack::Empty | Stack::E => VariableSet::new_singleton(self.v_var),
            _ => VariableSet::new_empty(),
        }
    }

    fn estimate(&self, variable: VariableId) -> u32 {
        match self.state {
            Stack::Empty if variable == self.e_var => self.eav_cursor.count_segment(),
            Stack::Empty if variable == self.a_var => self.aev_cursor.count_segment(),
            Stack::E if variable == self.a_var => self.eav_cursor.count_segment(),
            Stack::EA if variable == self.v_var => self.eav_cursor.count_segment(),
            Stack::A if variable == self.e_var => self.aev_cursor.count_segment(),
            Stack::A if variable == self.v_var => self.ave_cursor.count_segment(),
            Stack::AE if variable == self.v_var => self.aev_cursor.count_segment(),
            Stack::AV if variable == self.e_var => self.ave_cursor.count_segment(),
            _ => panic!("unreachable"),
        }
    }

    fn explore(&mut self, variable: VariableId) {
        match self.state {
            Stack::Empty if variable == self.e_var => self.state = Stack::E,
            Stack::Empty if variable == self.a_var => self.state = Stack::A,
            Stack::E if variable == self.a_var => self.state = Stack::EA,
            Stack::EA if variable == self.v_var => self.state = Stack::EAV,
            Stack::A if variable == self.e_var => self.state = Stack::AE,
            Stack::A if variable == self.v_var => self.state = Stack::AV,
            Stack::AE if variable == self.v_var => self.state = Stack::AEV,
            Stack::AV if variable == self.e_var => self.state = Stack::AVE,
            _ => panic!("unreachable"),
        }
    }

    fn retreat(&mut self) {
        match self.state {
            Stack::E | Stack::A => self.state = Stack::Empty,
            Stack::EA => self.state = Stack::E,
            Stack::EAV => self.state = Stack::EA,
            Stack::AE | Stack::AV => self.state = Stack::A,
            Stack::AEV => self.state = Stack::AE,
            Stack::AVE => self.state = Stack::AV,
            _ => panic!("unreachable"),
        }
    }
}
*/
