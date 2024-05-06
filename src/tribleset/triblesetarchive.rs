//mod triblesetarchiveconstraint;

use bytes::Bytes;
use std::collections::HashSet;
use triblesetarchiveconstraint::*;

use crate::query::TriblePattern;

use crate::{id_into_value, Handle};
use crate::{BlobParseError, Bloblike, Id, Value, Valuelike};
use core::panic;

use itertools::Itertools;

use sucds::bit_vectors::DArray;
use sucds::char_sequences::WaveletMatrix;
use sucds::mii_sequences::EliasFano;
use sucds::int_vectors::CompactVector;

use super::TribleSet;

#[derive(Debug, Clone)]
pub struct TribleSetArchive {
    pub domain: Vec<Value>,

    pub fa_e: EliasFano,
    pub fa_a: EliasFano,
    pub fa_v: EliasFano,
    pub ra_v: EliasFano,
    pub ra_a: EliasFano,
    pub ra_e: EliasFano,

    pub fc_e: WaveletMatrix<DArray>,
    pub fc_a: WaveletMatrix<DArray>,
    pub fc_v: WaveletMatrix<DArray>,
    pub rc_v: WaveletMatrix<DArray>,
    pub rc_a: WaveletMatrix<DArray>,
    pub rc_e: WaveletMatrix<DArray>,
}

impl From<&TribleSet> for TribleSetArchive {
    fn from(set: &TribleSet) -> Self {
        let e_iter = set.eav.iter_prefix::<16>().map(|(e, _)| id_into_value(e));
        let a_iter = set.ave.iter_prefix::<16>().map(|(a, _)| id_into_value(a));
        let v_iter = set.vea.iter_prefix::<32>().map(|(v, _)| v);

        let domain: Vec<Value> = e_iter.merge(a_iter).merge(v_iter).dedup().collect();

        TribleSetArchive {
            domain,
            fa_e: todo!(),
            fa_a: todo!(),
            fa_v: todo!(),
            ra_v: todo!(),
            ra_a: todo!(),
            ra_e: todo!(),
            fc_e: todo!(),
            fc_a: todo!(),
            fc_v: todo!(),
            rc_v: todo!(),
            rc_a: todo!(),
            rc_e: todo!(),
        }

    }
}

/*
impl TriblePattern for TribleSetArchive {
    type PatternConstraint<'a, V>
     = TribleSetArchiveConstraint<'a, V>
     where V: Valuelike;

    fn pattern<'a, V>(
        &'a self,
        e: crate::query::Variable<Id>,
        a: crate::query::Variable<Id>,
        v: crate::query::Variable<V>,
    ) -> Self::PatternConstraint<'a, V>
    where
        V: Valuelike,
    {
        TribleSetArchiveConstraint::new(e, a, v, self)
    }
}
*/

impl<'a> Bloblike<'a> for TribleSetArchive {
    type Read = TribleSetArchive;

    fn read_blob(blob: &Bytes) -> Result<Self::Read, BlobParseError> {
        todo!()
    }

    fn into_blob(self) -> Bytes {
        todo!()
    }

    fn as_handle<H>(&self) -> Handle<H, Self>
    where
        H: digest::Digest + digest::OutputSizeUser<OutputSize = digest::consts::U32>,
    {
        todo!()
    }
}
