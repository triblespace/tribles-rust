use core::panic;
use std::ops::Not;
use std::ops::Range;
//use std::convert::TryInto;
//use std::{collections::HashSet, fmt::Debug, hash::Hash};

use super::*;
use crate::id_from_value;
use crate::id_into_value;
use crate::query::*;
use crate::Id;
use crate::Valuelike;
use crate::ID_LEN;
use crate::VALUE_LEN;

pub struct TribleArchiveConstraint<'a, V, U, B>
where
    V: Valuelike,
    U: Universe,
    B: Build + Access + Rank + Select + NumBits 
{
    variable_e: Variable<Id>,
    variable_a: Variable<Id>,
    variable_v: Variable<V>,
    archive: &'a TribleArchive<U, B>,
}

impl<'a, V, U, B> TribleArchiveConstraint<'a, V, U, B>
where
    V: Valuelike,
    U: Universe,
    B: Build + Access + Rank + Select + NumBits 
{
    pub fn new(
        variable_e: Variable<Id>,
        variable_a: Variable<Id>,
        variable_v: Variable<V>,
        archive: &'a TribleArchive<U, B>,
    ) -> Self {
        TribleArchiveConstraint {
            variable_e,
            variable_a,
            variable_v,
            archive,
        }
    }
}

fn base_range<U>(universe: &U, a: &EliasFano, value: &Value) -> Option<Range<usize>>
where U: Universe {
    let d = universe.search(value)?;
    let s = a.rank(d)?;
    let e = a.rank(d + 1)?;
    Some(s..e)
}

fn restrict_range<U, B>(universe: &U, a: &EliasFano, c: &WaveletMatrix<B>, value: &Value, r: &Range<usize>) -> Option<Range<usize>>
where U: Universe,
    B: Build + Access + Rank + Select + NumBits {
    let s = r.start;
    let e = r.end;
    let d = universe.search(value)?;
    
    let s_ = a.rank(d)? + c.rank(s, d)?;
    let e_ = a.rank(d)? + c.rank(e, d)?;
    Some(s_..e_)
}

impl<'a, V, U, B> Constraint<'a> for TribleArchiveConstraint<'a, V, U, B>
where
    V: Valuelike,
    U: Universe,
    B: Build + Access + Rank + Select + NumBits 
{
    fn variables(&self) -> VariableSet {
        let mut variables = VariableSet::new_empty();
        variables.set(self.variable_e.index);
        variables.set(self.variable_a.index);
        variables.set(self.variable_v.index);
        variables
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.variable_e.index == variable
            || self.variable_a.index == variable
            || self.variable_v.index == variable
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index);
        let a_bound = binding.get(self.variable_a.index);
        let v_bound = binding.get(self.variable_v.index);

        //TODO add disting color counting ds to archive and estimate better
        (match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => self.archive.e_a.len(),
            (None, None, None, false, true, false) => self.archive.a_a.len(),
            (None, None, None, false, false, true) => self.archive.v_a.len(),
            (Some(e), None, None, false, true, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).unwrap_or(0..0).len()
            }
            (Some(e), None, None, false, false, true) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).unwrap_or(0..0).len()
            }
            (None, Some(a), None, true, false, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &a
                ).unwrap_or(0..0).len()
            }
            (None, Some(a), None, false, false, true) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &a
                ).unwrap_or(0..0).len()
            }
            (None, None, Some(v), true, false, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &v
                ).unwrap_or(0..0).len()
            }
            (None, None, Some(v), false, true, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &v
                ).unwrap_or(0..0).len()
            }
            (None, Some(a), Some(v), true, false, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &a
                ).and_then(|r|
                    restrict_range(&self.archive.domain, &self.archive.v_a, &self.archive.aev_c, &v, &r)
                ).unwrap_or(0..0).len()
            }
            (Some(e), None, Some(v), false, true, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).and_then(|r|
                    restrict_range(&self.archive.domain, &self.archive.v_a, &self.archive.eav_c, &v, &r)
                ).unwrap_or(0..0).len()
            }
            (Some(e), Some(a), None, false, false, true) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).and_then(|r|
                    restrict_range(&self.archive.domain, &self.archive.a_a, &self.archive.eva_c, &a, &r)
                ).unwrap_or(0..0).len()
            }
            _ => panic!(),
        }) as usize
    }

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<Value> {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index);
        let a_bound = binding.get(self.variable_a.index);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) =>
                self.archive.e_a.iter(0).dedup().map(|e| self.archive.domain.access(e)).collect(),
            (None, None, None, false, true, false) =>
                self.archive.a_a.iter(0).dedup().map(|a| self.archive.domain.access(a)).collect(),
            (None, None, None, false, false, true) =>
                self.archive.v_a.iter(0).dedup().map(|v| self.archive.domain.access(v)).collect(),
            (Some(e), None, None, false, true, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).unwrap_or(0..0)
                .map(|e| self.archive.eva_c.access(e).unwrap())
                .unique()
                .map(|a| self.archive.domain.access(a))
                .collect()
            }
            (Some(e), None, None, false, false, true) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).unwrap_or(0..0)
                .map(|v| self.archive.eav_c.access(v).unwrap())
                .unique()
                .map(|v| self.archive.domain.access(v))
                .collect()
            }

            (None, Some(a), None, true, false, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &a
                ).unwrap_or(0..0)
                .map(|e| self.archive.ave_c.access(e).unwrap())
                .unique()
                .map(|e| self.archive.domain.access(e))
                .collect()
            }
            (None, Some(a), None, false, false, true) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &a
                ).unwrap_or(0..0)
                .map(|v| self.archive.aev_c.access(v).unwrap())
                .unique()
                .map(|v| self.archive.domain.access(v))
                .collect()
            }

            (None, None, Some(v), true, false, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &v
                ).unwrap_or(0..0)
                .map(|e| self.archive.vae_c.access(e).unwrap())
                .unique()
                .map(|e| self.archive.domain.access(e))
                .collect()
            }
            (None, None, Some(v), false, true, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &v
                ).unwrap_or(0..0)
                .map(|a| self.archive.vea_c.access(a).unwrap())
                .unique()
                .map(|a| self.archive.domain.access(a))
                .collect()
            }
            (None, Some(a), Some(v), true, false, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &a
                ).and_then(|r|
                    restrict_range(&self.archive.domain, &self.archive.v_a, &self.archive.aev_c, &v, &r))
                .unwrap_or(0..0)
                .map(|e| self.archive.vae_c.access(e).unwrap())
                .unique()
                .map(|e| self.archive.domain.access(e))
                .collect()
            }
            (Some(e), None, Some(v), false, true, false) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).and_then(|r|
                    restrict_range(&self.archive.domain, &self.archive.v_a, &self.archive.eav_c, &v, &r))
                .unwrap_or(0..0)
                .map(|a| self.archive.vea_c.access(a).unwrap())
                .unique()
                .map(|a| self.archive.domain.access(a))
                .collect()
            }
            (Some(e), Some(a), None, false, false, true) => {
                base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).and_then(|r|
                    restrict_range(&self.archive.domain, &self.archive.a_a, &self.archive.eva_c, &a, &r))
                .unwrap_or(0..0)
                .map(|v| self.archive.aev_c.access(v).unwrap())
                .unique()
                .map(|v| self.archive.domain.access(v))
                .collect()
            }
            _ => panic!(),
        }
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<Value>) {
        let e_var = self.variable_e.index == variable;
        let a_var = self.variable_a.index == variable;
        let v_var = self.variable_v.index == variable;

        let e_bound = binding.get(self.variable_e.index).map(id_from_value);
        let a_bound = binding.get(self.variable_a.index).map(id_from_value);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => {
                proposals.retain(|e| base_range(
                    &self.archive.domain,
                    &self.archive.e_a,
                    &e
                ).unwrap_or(0..0).is_empty().not())
            }
            (None, None, None, false, true, false) => {
                proposals.retain(|a| base_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &a
                ).unwrap_or(0..0).is_empty().not())            }
            (None, None, None, false, false, true) => {
                proposals.retain(|v| base_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &v
                ).unwrap_or(0..0).is_empty().not())            }
            (Some(e), None, None, false, true, false) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&id_from_value(*value));
                self.set.eav.has_prefix(&prefix)
            }),
            (Some(e), None, None, false, false, true) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(value);
                self.set.eva.has_prefix(&prefix)
            }),
            (None, Some(a), None, true, false, false) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&id_from_value(*value));
                self.set.aev.has_prefix(&prefix)
            }),
            (None, Some(a), None, false, false, true) => proposals.retain(|value| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a[..]);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(value);
                self.set.ave.has_prefix(&prefix)
            }),
            (None, None, Some(v), true, false, false) => proposals.retain(|value| {
                let mut prefix = [0u8; VALUE_LEN + ID_LEN];
                prefix[0..VALUE_LEN].copy_from_slice(&v[..]);
                prefix[VALUE_LEN..VALUE_LEN + ID_LEN].copy_from_slice(&id_from_value(*value));
                self.set.vea.has_prefix(&prefix)
            }),
            (None, None, Some(v), false, true, false) => proposals.retain(|value| {
                let mut prefix = [0u8; VALUE_LEN + ID_LEN];
                prefix[0..VALUE_LEN].copy_from_slice(&v[..]);
                prefix[VALUE_LEN..VALUE_LEN + ID_LEN].copy_from_slice(&id_from_value(*value));
                self.set.vae.has_prefix(&prefix)
            }),
            (None, Some(a), Some(v), true, false, false) => proposals.retain(|value: &[u8; 32]| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&a);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v);
                prefix[ID_LEN + VALUE_LEN..ID_LEN + VALUE_LEN + ID_LEN]
                    .copy_from_slice(&id_from_value(*value));
                self.set.ave.has_prefix(&prefix)
            }),
            (Some(e), None, Some(v), false, true, false) => proposals.retain(|value: &[u8; 32]| {
                let mut prefix = [0u8; ID_LEN + VALUE_LEN + ID_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e);
                prefix[ID_LEN..ID_LEN + VALUE_LEN].copy_from_slice(&v);
                prefix[ID_LEN + VALUE_LEN..ID_LEN + VALUE_LEN + ID_LEN]
                    .copy_from_slice(&id_from_value(*value));
                self.set.eva.has_prefix(&prefix)
            }),
            (Some(e), Some(a), None, false, false, true) => proposals.retain(|value: &[u8; 32]| {
                let mut prefix = [0u8; ID_LEN + ID_LEN + VALUE_LEN];
                prefix[0..ID_LEN].copy_from_slice(&e);
                prefix[ID_LEN..ID_LEN + ID_LEN].copy_from_slice(&a);
                prefix[ID_LEN + ID_LEN..ID_LEN + ID_LEN + VALUE_LEN].copy_from_slice(value);
                self.set.eav.has_prefix(&prefix)
            }),
            _ => panic!("invalid trible constraint state"),
        }
    }
}
