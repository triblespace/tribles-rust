use core::panic;
use std::convert::TryInto;
use std::{collections::HashSet, fmt::Debug, hash::Hash};

use super::*;
use crate::namespace::*;
use crate::query::*;
use crate::trible::*;

pub struct PACTTribleSetConstraint<'a, E, A, V>
where
    E: From<Id>,
    A: From<Id>,
    V: From<Value>,
    for<'b> &'b E: Into<Id>,
    for<'b> &'b A: Into<Id>,
    for<'b> &'b V: Into<Value>,
{
    variable_e: Variable<E>,
    variable_a: Variable<A>,
    variable_v: Variable<V>,
    set: &'a PACTTribleSet,
}

impl<'a, E, A, V> PACTTribleSetConstraint<'a, E, A, V>
where
    E: From<Id>,
    A: From<Id>,
    V: From<Value>,
    for<'b> &'b E: Into<Id>,
    for<'b> &'b A: Into<Id>,
    for<'b> &'b V: Into<Value>,
{
    pub fn new(
        variable_e: Variable<E>,
        variable_a: Variable<A>,
        variable_v: Variable<V>,
        set: &'a PACTTribleSet,
    ) -> Self {
        PACTTribleSetConstraint {
            variable_e,
            variable_a,
            variable_v,
            set,
        }
    }
}

impl<'a, E, A, V> Constraint<'a> for PACTTribleSetConstraint<'a, E, A, V>
where
    E: From<Id>,
    A: From<Id>,
    V: From<Value>,
    for<'b> &'b E: Into<Id>,
    for<'b> &'b A: Into<Id>,
    for<'b> &'b V: Into<Value>,
{
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e.index);
        variables.set(self.variable_a.index);
        variables.set(self.variable_v.index);
        variables
    }

    fn estimate(&self, variable: VariableId, binding: Binding) -> usize {
        let e_bound = binding.bound.is_set(self.variable_e.index);
        let a_bound = binding.bound.is_set(self.variable_a.index);
        let v_bound = binding.bound.is_set(self.variable_v.index);

        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        if let Some(trible) = Trible::raw_values(
            binding.get(self.variable_e.index).unwrap_or([0; 32]),
            binding.get(self.variable_a.index).unwrap_or([0; 32]),
            binding.get(self.variable_v.index).unwrap_or([0; 32]),
        ) {
            match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
                (false, false, false, true, false, false) => {
                    self.set.eav.segmented_len(trible.data, E_START)
                }
                (false, false, false, false, true, false) => {
                    self.set.aev.segmented_len(trible.data, A_START)
                }
                (false, false, false, false, false, true) => {
                    self.set.vea.segmented_len(trible.data, V_START)
                }

                (true, false, false, false, true, false) => {
                    self.set.eav.segmented_len(trible.data, A_START)
                }
                (true, false, false, false, false, true) => {
                    self.set.eva.segmented_len(trible.data, V_START)
                }

                (false, true, false, true, false, false) => {
                    self.set.aev.segmented_len(trible.data, E_START)
                }
                (false, true, false, false, false, true) => {
                    self.set.ave.segmented_len(trible.data, V_START)
                }

                (false, false, true, true, false, false) => {
                    self.set.vea.segmented_len(trible.data, E_START)
                }
                (false, false, true, false, true, false) => {
                    self.set.vae.segmented_len(trible.data, A_START)
                }

                (false, true, true, true, false, false) => {
                    self.set.ave.segmented_len(trible.data, E_START)
                }
                (true, false, true, false, true, false) => {
                    self.set.eva.segmented_len(trible.data, A_START)
                }
                (true, true, false, false, false, true) => {
                    self.set.eav.segmented_len(trible.data, V_START)
                }
                _ => panic!(),
            }
        } else {
            0
        }
    }

    fn propose(&self, variable: VariableId, binding: Binding) -> Vec<Value> {
        let e_bound = binding.bound.is_set(self.variable_e.index);
        let a_bound = binding.bound.is_set(self.variable_a.index);
        let v_bound = binding.bound.is_set(self.variable_v.index);

        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        if let Some(trible) = Trible::raw_values(
            binding.get(self.variable_e.index).unwrap_or([0; 32]),
            binding.get(self.variable_a.index).unwrap_or([0; 32]),
            binding.get(self.variable_v.index).unwrap_or([0; 32]),
        ) {
            match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
                (false, false, false, true, false, false) => {
                    self.set
                        .eav
                        .infixes(trible.data, E_START, E_END, |k| Trible::raw(k).e_as_value())
                }
                (false, false, false, false, true, false) => {
                    self.set
                        .aev
                        .infixes(trible.data, A_START, A_END, |k| Trible::raw(k).a_as_value())
                }
                (false, false, false, false, false, true) => {
                    self.set
                        .vea
                        .infixes(trible.data, V_START, V_END, |k| Trible::raw(k).v())
                }

                (true, false, false, false, true, false) => {
                    self.set
                        .eav
                        .infixes(trible.data, A_START, A_END, |k| Trible::raw(k).a_as_value())
                }
                (true, false, false, false, false, true) => {
                    self.set
                        .eva
                        .infixes(trible.data, V_START, V_END, |k| Trible::raw(k).v())
                }

                (false, true, false, true, false, false) => {
                    self.set
                        .aev
                        .infixes(trible.data, E_START, E_END, |k| Trible::raw(k).e_as_value())
                }
                (false, true, false, false, false, true) => {
                    self.set
                        .ave
                        .infixes(trible.data, V_START, V_END, |k| Trible::raw(k).v())
                }

                (false, false, true, true, false, false) => {
                    self.set
                        .vea
                        .infixes(trible.data, E_START, E_END, |k| Trible::raw(k).e_as_value())
                }
                (false, false, true, false, true, false) => {
                    self.set
                        .vae
                        .infixes(trible.data, A_START, A_END, |k| Trible::raw(k).a_as_value())
                }

                (false, true, true, true, false, false) => {
                    self.set
                        .ave
                        .infixes(trible.data, E_START, E_END, |k| Trible::raw(k).e_as_value())
                }
                (true, false, true, false, true, false) => {
                    self.set
                        .eva
                        .infixes(trible.data, A_START, A_END, |k| Trible::raw(k).a_as_value())
                }
                (true, true, false, false, false, true) => {
                    self.set
                        .eav
                        .infixes(trible.data, V_START, V_END, |k| Trible::raw(k).v())
                }
                _ => panic!(),
            }
        } else {
            vec![]
        }
    }

    fn confirm(&self, variable: VariableId, binding: Binding, proposals: &mut Vec<Value>) {
        let e_bound = binding.bound.is_set(self.variable_e.index);
        let a_bound = binding.bound.is_set(self.variable_a.index);
        let v_bound = binding.bound.is_set(self.variable_v.index);

        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (false, false, false, true, false, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.eav.has_prefix(trible.data, E_END)
                    } else {
                        false
                    }
                });
            }
            (false, false, false, false, true, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.aev.has_prefix(trible.data, A_END)
                    } else {
                        false
                    }
                });
            }
            (false, false, false, false, false, true) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.vea.has_prefix(trible.data, V_END)
                    } else {
                        false
                    }
                });
            }

            (true, false, false, false, true, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.eav.has_prefix(trible.data, A_END)
                    } else {
                        false
                    }
                });
            }
            (true, false, false, false, false, true) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.eva.has_prefix(trible.data, V_END)
                    } else {
                        false
                    }
                });
            }

            (false, true, false, true, false, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.aev.has_prefix(trible.data, E_END)
                    } else {
                        false
                    }
                });
            }

            (false, true, false, false, false, true) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.ave.has_prefix(trible.data, V_END)
                    } else {
                        false
                    }
                });
            }

            (false, false, true, true, false, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.vea.has_prefix(trible.data, E_END)
                    } else {
                        false
                    }
                });
            }
            (false, false, true, false, true, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.vae.has_prefix(trible.data, A_END)
                    } else {
                        false
                    }
                });
            }

            (false, true, true, true, false, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.ave.has_prefix(trible.data, E_END)
                    } else {
                        false
                    }
                });
            }
            (true, false, true, false, true, false) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.eva.has_prefix(trible.data, A_END)
                    } else {
                        false
                    }
                });
            }
            (true, true, false, false, false, true) => {
                proposals.retain(|value| {
                    if let Some(trible) = Trible::raw_values(
                        binding.get(self.variable_e.index).unwrap_or(if e_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_a.index).unwrap_or(if a_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                        binding.get(self.variable_v.index).unwrap_or(if v_var {
                            *value
                        } else {
                            [0; 32]
                        }),
                    ) {
                        self.set.eav.has_prefix(trible.data, V_END)
                    } else {
                        false
                    }
                });
            }
            _ => panic!(),
        }
    }
}
