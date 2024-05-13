use core::panic;
use std::ops::Not;
use std::ops::Range;
//use std::convert::TryInto;
//use std::{collections::HashSet, fmt::Debug, hash::Hash};

use super::*;
use crate::query::*;
use crate::Id;
use crate::Valuelike;

pub struct TribleArchiveConstraint<'a, V, U, B>
where
    V: Valuelike,
    U: Universe,
    B: Build + Access + Rank + Select + NumBits,
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
    B: Build + Access + Rank + Select + NumBits,
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

fn base_range<U>(universe: &U, a: &EliasFano, value: &Value) -> Range<usize>
where
    U: Universe,
{
    if let Some(d) = universe.search(value) {
        let s = a.select(d).unwrap();
        let e = a.select(d + 1).unwrap();
        s..e
    } else {
        0..0
    }
}

fn restrict_range<U, B>(
    universe: &U,
    a: &EliasFano,
    c: &WaveletMatrix<B>,
    value: &Value,
    r: &Range<usize>,
) -> Range<usize>
where
    U: Universe,
    B: Build + Access + Rank + Select + NumBits,
{
    let s = r.start;
    let e = r.end;
    if let Some(d) = universe.search(value) {
        let s_ = a.select(d).unwrap() + c.rank(s, d).unwrap();
        let e_ = a.select(d).unwrap() + c.rank(e, d).unwrap();
        s_..e_
    } else {
        0..0
    }
}

impl<'a, V, U, B> Constraint<'a> for TribleArchiveConstraint<'a, V, U, B>
where
    V: Valuelike,
    U: Universe,
    B: Build + Access + Rank + Select + NumBits,
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
                base_range(&self.archive.domain, &self.archive.e_a, &e).len()
            }
            (Some(e), None, None, false, false, true) => {
                base_range(&self.archive.domain, &self.archive.e_a, &e).len()
            }
            (None, Some(a), None, true, false, false) => {
                base_range(&self.archive.domain, &self.archive.a_a, &a).len()
            }
            (None, Some(a), None, false, false, true) => {
                base_range(&self.archive.domain, &self.archive.a_a, &a).len()
            }
            (None, None, Some(v), true, false, false) => {
                base_range(&self.archive.domain, &self.archive.v_a, &v).len()
            }
            (None, None, Some(v), false, true, false) => {
                base_range(&self.archive.domain, &self.archive.v_a, &v).len()
            }
            (None, Some(a), Some(v), true, false, false) => {
                let r = base_range(&self.archive.domain, &self.archive.a_a, &a);
                let r = restrict_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &self.archive.aev_c,
                    &v,
                    &r,
                );
                r.len()
            }
            (Some(e), None, Some(v), false, true, false) => {
                let r = base_range(&self.archive.domain, &self.archive.e_a, &e);
                let r = restrict_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &self.archive.eav_c,
                    &v,
                    &r,
                );
                r.len()
            }
            (Some(e), Some(a), None, false, false, true) => {
                let r = base_range(&self.archive.domain, &self.archive.e_a, &e);
                let r = restrict_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &self.archive.eva_c,
                    &a,
                    &r,
                );
                r.len()
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
            (None, None, None, true, false, false) => self
                .archive
                .e_a
                .iter(0)
                .dedup()
                .map(|e| self.archive.domain.access(e))
                .collect(),
            (None, None, None, false, true, false) => self
                .archive
                .a_a
                .iter(0)
                .dedup()
                .map(|a| self.archive.domain.access(a))
                .collect(),
            (None, None, None, false, false, true) => self
                .archive
                .v_a
                .iter(0)
                .dedup()
                .map(|v| self.archive.domain.access(v))
                .collect(),
            (Some(e), None, None, false, true, false) => {
                base_range(&self.archive.domain, &self.archive.e_a, &e)
                    .map(|e| self.archive.eva_c.access(e).unwrap())
                    .unique()
                    .map(|a| self.archive.domain.access(a))
                    .collect()
            }
            (Some(e), None, None, false, false, true) => {
                base_range(&self.archive.domain, &self.archive.e_a, &e)
                    .map(|v| self.archive.eav_c.access(v).unwrap())
                    .unique()
                    .map(|v| self.archive.domain.access(v))
                    .collect()
            }

            (None, Some(a), None, true, false, false) => {
                base_range(&self.archive.domain, &self.archive.a_a, &a)
                    .map(|e| self.archive.ave_c.access(e).unwrap())
                    .unique()
                    .map(|e| self.archive.domain.access(e))
                    .collect()
            }
            (None, Some(a), None, false, false, true) => {
                base_range(&self.archive.domain, &self.archive.a_a, &a)
                    .map(|v| self.archive.aev_c.access(v).unwrap())
                    .unique()
                    .map(|v| self.archive.domain.access(v))
                    .collect()
            }

            (None, None, Some(v), true, false, false) => {
                base_range(&self.archive.domain, &self.archive.v_a, &v)
                    .map(|e| self.archive.vae_c.access(e).unwrap())
                    .unique()
                    .map(|e| self.archive.domain.access(e))
                    .collect()
            }
            (None, None, Some(v), false, true, false) => {
                base_range(&self.archive.domain, &self.archive.v_a, &v)
                    .map(|a| self.archive.vea_c.access(a).unwrap())
                    .unique()
                    .map(|a| self.archive.domain.access(a))
                    .collect()
            }
            (None, Some(a), Some(v), true, false, false) => {
                let r = base_range(&self.archive.domain, &self.archive.a_a, &a);
                restrict_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &self.archive.aev_c,
                    &v,
                    &r,
                )
                .map(|e| self.archive.vae_c.access(e).unwrap())
                .unique()
                .map(|e| self.archive.domain.access(e))
                .collect()
            }
            (Some(e), None, Some(v), false, true, false) => {
                let r = base_range(&self.archive.domain, &self.archive.e_a, &e);
                restrict_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &self.archive.eav_c,
                    &v,
                    &r,
                )
                .map(|a| self.archive.vea_c.access(a).unwrap())
                .unique()
                .map(|a| self.archive.domain.access(a))
                .collect()
            }
            (Some(e), Some(a), None, false, false, true) => {
                let r = base_range(&self.archive.domain, &self.archive.e_a, &e);
                restrict_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &self.archive.eva_c,
                    &a,
                    &r,
                )
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

        let e_bound = binding.get(self.variable_e.index);
        let a_bound = binding.get(self.variable_a.index);
        let v_bound = binding.get(self.variable_v.index);

        match (e_bound, a_bound, v_bound, e_var, a_var, v_var) {
            (None, None, None, true, false, false) => {
                proposals.retain(|e| {
                    base_range(&self.archive.domain, &self.archive.e_a, &e)
                        .is_empty()
                        .not()
                });
            }
            (None, None, None, false, true, false) => {
                proposals.retain(|a| {
                    base_range(&self.archive.domain, &self.archive.a_a, &a)
                        .is_empty()
                        .not()
                });
            }
            (None, None, None, false, false, true) => {
                proposals.retain(|v| {
                    base_range(&self.archive.domain, &self.archive.v_a, &v)
                        .is_empty()
                        .not()
                });
            }
            (Some(e), None, None, false, true, false) => {
                let r = base_range(&self.archive.domain, &self.archive.e_a, &e);
                proposals.retain(|a| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.a_a,
                        &self.archive.eva_c,
                        &a,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            (Some(e), None, None, false, false, true) => {
                let r = base_range(&self.archive.domain, &self.archive.e_a, &e);
                proposals.retain(|v| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.v_a,
                        &self.archive.eav_c,
                        &v,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            (None, Some(a), None, true, false, false) => {
                let r = base_range(&self.archive.domain, &self.archive.a_a, &a);
                proposals.retain(|e| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.e_a,
                        &self.archive.ave_c,
                        &e,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            (None, Some(a), None, false, false, true) => {
                let r = base_range(&self.archive.domain, &self.archive.a_a, &a);
                proposals.retain(|v| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.v_a,
                        &self.archive.aev_c,
                        &v,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            (None, None, Some(v), true, false, false) => {
                let r = base_range(&self.archive.domain, &self.archive.v_a, &v);
                proposals.retain(|e| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.e_a,
                        &self.archive.vae_c,
                        &e,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            (None, None, Some(v), false, true, false) => {
                let r = base_range(&self.archive.domain, &self.archive.v_a, &v);
                proposals.retain(|a| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.a_a,
                        &self.archive.vea_c,
                        &a,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            (None, Some(a), Some(v), true, false, false) => {
                let r = base_range(&self.archive.domain, &self.archive.a_a, &a);
                let r = restrict_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &self.archive.aev_c,
                    &v,
                    &r,
                );
                proposals.retain(|e| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.e_a,
                        &self.archive.vae_c,
                        &e,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            (Some(e), None, Some(v), false, true, false) => {
                let r = base_range(&self.archive.domain, &self.archive.e_a, &e);
                let r = restrict_range(
                    &self.archive.domain,
                    &self.archive.v_a,
                    &self.archive.eav_c,
                    &v,
                    &r,
                );
                proposals.retain(|a| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.a_a,
                        &self.archive.vea_c,
                        &a,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            (Some(e), Some(a), None, false, false, true) => {
                let r = base_range(&self.archive.domain, &self.archive.e_a, &e);
                let r = restrict_range(
                    &self.archive.domain,
                    &self.archive.a_a,
                    &self.archive.eva_c,
                    &a,
                    &r,
                );
                proposals.retain(|v| {
                    restrict_range(
                        &self.archive.domain,
                        &self.archive.v_a,
                        &self.archive.aev_c,
                        &v,
                        &r,
                    )
                    .is_empty()
                    .not()
                });
            }
            _ => panic!("invalid trible constraint state"),
        }
    }
}
