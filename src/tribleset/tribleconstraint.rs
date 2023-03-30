use std::rc::Rc;

use crate::pact::PaddedCursor;
use crate::query::*;
use crate::trible::{AEVOrder, AVEOrder, EAVOrder, EVAOrder, VAEOrder, VEAOrder};

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
    eav_cursor: Rc<PaddedCursor<64, EAVOrder>>,
    eva_cursor: Rc<PaddedCursor<64, EVAOrder>>,
    aev_cursor: Rc<PaddedCursor<64, AEVOrder>>,
    ave_cursor: Rc<PaddedCursor<64, AVEOrder>>,
    vea_cursor: Rc<PaddedCursor<64, VEAOrder>>,
    vae_cursor: Rc<PaddedCursor<64, VAEOrder>>,
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
            eav_cursor: Rc::new(set.eav.padded_cursor()),
            eva_cursor: Rc::new(set.eva.padded_cursor()),
            aev_cursor: Rc::new(set.aev.padded_cursor()),
            ave_cursor: Rc::new(set.ave.padded_cursor()),
            vea_cursor: Rc::new(set.vea.padded_cursor()),
            vae_cursor: Rc::new(set.vae.padded_cursor()),
        }
    }
}

struct CoupledCursor {
    primary: Rc<dyn ByteCursor>,
    secondary: Rc<dyn ByteCursor>,
}

impl CoupledCursor {
    fn new(primary: Rc<dyn ByteCursor>, secondary: Rc<dyn ByteCursor>) -> Rc<Self> {
        Rc::new(Self { primary, secondary })
    }
}

impl ByteCursor for CoupledCursor {
    fn peek(&self) -> Peek {
        self.primary.peek()
    }

    fn push(&mut self, byte: u8) {
        self.primary.push(byte);
        self.secondary.push(byte);
    }

    fn pop(&mut self) {
        self.primary.pop();
        self.secondary.pop();
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
        VariableSet::new_empty()
    }

    fn estimate(&self, variable: VariableId) -> u32 {
        match self.state {
            Stack::Empty if variable == self.e_var => self.eav_cursor.count_segment(),
            Stack::Empty if variable == self.a_var => self.aev_cursor.count_segment(),
            Stack::Empty if variable == self.v_var => self.vea_cursor.count_segment(),
            Stack::E if variable == self.a_var => self.eav_cursor.count_segment(),
            Stack::E if variable == self.v_var => self.eva_cursor.count_segment(),
            Stack::EA if variable == self.v_var => self.eav_cursor.count_segment(),
            Stack::EV if variable == self.a_var => self.eva_cursor.count_segment(),
            Stack::A if variable == self.e_var => self.aev_cursor.count_segment(),
            Stack::A if variable == self.v_var => self.ave_cursor.count_segment(),
            Stack::AE if variable == self.v_var => self.aev_cursor.count_segment(),
            Stack::AV if variable == self.e_var => self.ave_cursor.count_segment(),
            Stack::V if variable == self.e_var => self.vea_cursor.count_segment(),
            Stack::V if variable == self.a_var => self.vae_cursor.count_segment(),
            Stack::VE if variable == self.a_var => self.vea_cursor.count_segment(),
            Stack::VA if variable == self.e_var => self.vae_cursor.count_segment(),
            _ => panic!("unreachable"),
        }
    }

    fn explore(&mut self, variable: VariableId) -> Rc<dyn ByteCursor> {
        match self.state {
            Stack::Empty if variable == self.e_var => {
                self.state = Stack::E;
                CoupledCursor::new(self.eav_cursor, self.eva_cursor)
            }
            Stack::Empty if variable == self.a_var => {
                self.state = Stack::A;
                CoupledCursor::new(self.aev_cursor, self.ave_cursor)
            }
            Stack::Empty if variable == self.v_var => {
                self.state = Stack::V;
                CoupledCursor::new(self.vea_cursor, self.vae_cursor)
            }
            Stack::E if variable == self.a_var => {
                self.state = Stack::EA;
                self.eav_cursor
            }
            Stack::E if variable == self.v_var => {
                self.state = Stack::EV;
                self.eva_cursor
            }
            Stack::EA if variable == self.v_var => {
                self.state = Stack::EAV;
                self.eav_cursor
            }
            Stack::EV if variable == self.a_var => {
                self.state = Stack::EVA;
                self.eva_cursor
            }
            Stack::A if variable == self.e_var => {
                self.state = Stack::AE;
                self.aev_cursor
            }
            Stack::A if variable == self.v_var => {
                self.state = Stack::AV;
                self.ave_cursor
            }
            Stack::AE if variable == self.v_var => {
                self.state = Stack::AEV;
                self.aev_cursor
            }
            Stack::AV if variable == self.e_var => {
                self.state = Stack::AVE;
                self.ave_cursor
            }
            Stack::V if variable == self.e_var => {
                self.state = Stack::VE;
                self.vea_cursor
            }
            Stack::V if variable == self.a_var => {
                self.state = Stack::VA;
                self.vae_cursor
            }
            Stack::VE if variable == self.a_var => {
                self.state = Stack::VEA;
                self.vea_cursor
            }
            Stack::VA if variable == self.e_var => {
                self.state = Stack::VAE;
                self.vae_cursor
            }
            _ => panic!("unreachable"),
        }
    }

    fn retreat(&mut self, variable: VariableId) {
        match self.state {
            Stack::E | Stack::A | Stack::V => self.state = Stack::Empty,
            Stack::EA | Stack::EV => self.state = Stack::E,
            Stack::EAV => self.state = Stack::EA,
            Stack::EVA => self.state = Stack::EV,
            Stack::AE | Stack::AV => self.state = Stack::A,
            Stack::AEV => self.state = Stack::AE,
            Stack::AVE => self.state = Stack::AV,
            Stack::VE | Stack::VA => self.state = Stack::V,
            Stack::VEA => self.state = Stack::VE,
            Stack::VAE => self.state = Stack::VA,
            _ => panic!("unreachable"),
        }
    }
}
