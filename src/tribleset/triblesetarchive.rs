//mod triblesetarchiveconstraint;

use bytes::Bytes;
use std::collections::HashSet;
use std::convert::TryInto;
use std::iter;
use triblesetarchiveconstraint::*;

use crate::query::TriblePattern;

use crate::{id_into_value, Handle};
use crate::{BlobParseError, Bloblike, Id, Value, Valuelike};
use core::panic;

use itertools::Itertools;

use sucds::bit_vectors::DArray;
use sucds::char_sequences::WaveletMatrix;
use sucds::mii_sequences::{EliasFano, EliasFanoBuilder};
use sucds::int_vectors::CompactVector;

use super::TribleSet;

#[derive(Debug, Clone)]
pub struct TribleSetArchive {
    pub domain: Vec<Value>,

    pub eav_c: WaveletMatrix<DArray>,
    pub vea_c: WaveletMatrix<DArray>,
    pub ave_c: WaveletMatrix<DArray>,
    pub vae_c: WaveletMatrix<DArray>,
    pub eva_c: WaveletMatrix<DArray>,
    pub aev_c: WaveletMatrix<DArray>,

    pub eav_a: EliasFano,
    pub vea_a: EliasFano,
    pub ave_a: EliasFano,
    pub vae_a: EliasFano,
    pub eva_a: EliasFano,
    pub aev_a: EliasFano,
}

impl From<&TribleSet> for TribleSetArchive {
    fn from(set: &TribleSet) -> Self {
        let triple_count = set.eav.len() as usize;
        assert!(triple_count > 0);

        let e_iter = set.eav.iter_prefix::<16>().map(|(e, _)| id_into_value(e));
        let a_iter = set.ave.iter_prefix::<16>().map(|(a, _)| id_into_value(a));
        let v_iter = set.vea.iter_prefix::<32>().map(|(v, _)| v);

        let domain: Vec<Value> = e_iter.merge(a_iter).merge(v_iter).dedup().collect();
        let alph_width = sucds::utils::needed_bits(domain.len()-1);
        
        let mut ave_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        ave_c.extend(set.ave.iter_prefix::<64>()
            .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
            .map(|e| domain.binary_search(&e).expect("e in domain")));
        let ave_c = WaveletMatrix::<DArray>::new(ave_c).unwrap();

        let mut ave_a = EliasFanoBuilder::new(domain.len(), triple_count).expect("|T| > 0");
        ave_a.extend(set.eva.iter_prefix::<16>()
            .map(|(e, count)| (id_into_value(e), count as usize))
            .map(|(e, count)| (domain.binary_search(&e).expect("e in domain"), count))
            .flat_map(|(e, count)| iter::repeat(e).take(count)));
        let ave_a = ave_a.build();


        TribleSetArchive {
            domain,
            eav_c: todo!(),
            vea_c: todo!(),
            ave_c,
            vae_c: todo!(),
            eva_c: todo!(),
            aev_c: todo!(),
            eav_a: todo!(),
            vea_a: todo!(),
            ave_a,
            vae_a: todo!(),
            eva_a: todo!(),
            aev_a: todo!(),
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
