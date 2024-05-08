//mod triblesetarchiveconstraint;

//use bytes::Bytes;
use std::convert::TryInto;
use std::iter;
use std::ops::Range;
use std::ops::RangeBounds;
//use triblesetarchiveconstraint::*;

//use crate::query::TriblePattern;

use crate::id_into_value;
use crate::Value;
use crate::VALUE_LEN;

use indxvec::Search;
use itertools::Itertools;

use sucds::int_vectors::DacsByte;
use sucds::int_vectors::DacsOpt;
use sucds::int_vectors::{ Build as IBuild, Access as IAccess, NumVals};
use sucds::bit_vectors::{Build, Access, Rank, Select, NumBits};
use sucds::char_sequences::WaveletMatrix;
use sucds::mii_sequences::{EliasFano, EliasFanoBuilder};
use sucds::int_vectors::CompactVector;
use sucds::Serializable;

use super::TribleSet;

pub trait Universe {
    fn with<I>(iter: I) -> Self
    where I: Iterator<Item = Value>;
    fn access(&self, pos: usize) -> Value;
    fn search(&self, v: &Value) -> Option<usize>;
    fn size_in_bytes(&self) -> usize;
    fn len(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct OrderedUniverse {
    values: Vec<Value>,
}

impl Universe for OrderedUniverse {
    fn with<I>(iter: I) -> Self
    where I: Iterator<Item = Value> {
        Self {
            values: iter.collect()
        }
    }

    fn access(&self, pos: usize) -> Value {
        self.values[pos]
    }

    fn search(&self, v: &Value) -> Option<usize> {
        self.values.binary_search(v).ok()
    }

    fn size_in_bytes(&self) -> usize {
        self.values.len() * VALUE_LEN
    }

    fn len(&self) -> usize {
        self.values.len()
    }
}

#[derive(Debug, Clone)]
pub struct CompressedUniverse<C> {
    segments: Vec<C>,
}

impl<C> Universe for CompressedUniverse<C>
where C: IBuild + IAccess + NumVals + Serializable {
    fn with<I>(iter: I) -> Self
    where I: Iterator<Item = Value> {
        let mut clms: Vec<Vec<usize>> = vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()];

        for value in iter {
            clms[0].push(usize::from_be_bytes(value[0..8].try_into().unwrap()));
            clms[1].push(usize::from_be_bytes(value[8..16].try_into().unwrap()));
            clms[2].push(usize::from_be_bytes(value[16..24].try_into().unwrap()));
            clms[3].push(usize::from_be_bytes(value[24..32].try_into().unwrap()));
        }

        let mut columns: Vec<C> = vec![];
        for clm in clms {
            let column = C::build_from_slice(&clm).unwrap();
            columns.push(column);
        }

        Self {
            segments: columns
        }
    }

    fn access(&self, pos: usize) -> Value {
        let mut v: Value = [0; 32];

        v[0..8].copy_from_slice(&(self.segments[0].access(pos).unwrap().to_be_bytes()));
        v[8..16].copy_from_slice(&(self.segments[1].access(pos).unwrap().to_be_bytes()));
        v[16..24].copy_from_slice(&(self.segments[2].access(pos).unwrap().to_be_bytes()));
        v[24..32].copy_from_slice(&(self.segments[3].access(pos).unwrap().to_be_bytes()));

        v
    }

    fn search(&self, v: &Value) -> Option<usize> {
        (0..=self.segments[0].num_vals()-1).binary_by(|p| self.access(p).cmp(v)).ok()
        /* //TODO fix this and bench.
        let v0 = usize::from_be_bytes(v[0..8].try_into().unwrap());
        let v1 = usize::from_be_bytes(v[8..16].try_into().unwrap());
        let v2 = usize::from_be_bytes(v[16..24].try_into().unwrap());
        let v3 = usize::from_be_bytes(v[24..32].try_into().unwrap());

        let r0 = (0..=self.segments[0].num_vals()-1).binary_all(|p| self.segments[0].access(p).unwrap().cmp(&v0));
        if r0.is_empty() { return None;}

        let r1 = (r0.start..=r0.end-1).binary_all(|p| self.segments[1].access(p).unwrap().cmp(&v1));
        if r1.is_empty() { return None;}

        let r2 = (r1.start..=r1.end-1).binary_all(|p| self.segments[2].access(p).unwrap().cmp(&v2));
        if r2.is_empty() { return None;}

        let r3 = (r2.start..=r2.end-1).binary_all(|p| self.segments[3].access(p).unwrap().cmp(&v3));
        if r3.is_empty() { return None;}

        assert!(r3.len() == 1);

        return Some(r3.start);
        */
    }

    fn size_in_bytes(&self) -> usize {
        self.segments.iter().map(|c| c.size_in_bytes()).sum()

    }

    fn len(&self) -> usize {
        self.segments[0].num_vals()
    }
}

#[derive(Debug, Clone)]
pub struct TribleSetArchive<U, B>
{
    pub domain: U,

    pub e_a: EliasFano,
    pub a_a: EliasFano,
    pub v_a: EliasFano,
    
    pub eav_c: WaveletMatrix<B>,
    pub vea_c: WaveletMatrix<B>,
    pub ave_c: WaveletMatrix<B>,
    pub vae_c: WaveletMatrix<B>,
    pub eva_c: WaveletMatrix<B>,
    pub aev_c: WaveletMatrix<B>,
}

impl<U, B> TribleSetArchive<U, B>
where 
    U: Universe,
    B: Build + Access + Rank + Select + NumBits {
    pub fn with(set: &TribleSet) -> Self {
        let triple_count = set.eav.len() as usize;
        assert!(triple_count > 0);

        let e_iter = set.eav.iter_prefix::<16>().map(|(e, _)| id_into_value(e));
        let a_iter = set.ave.iter_prefix::<16>().map(|(a, _)| id_into_value(a));
        let v_iter = set.vea.iter_prefix::<32>().map(|(v, _)| v);

        let domain = U::with(e_iter.merge(a_iter).merge(v_iter).dedup());
        let alph_width = sucds::utils::needed_bits(domain.len()-1);
        
        let mut e_a = EliasFanoBuilder::new(domain.len(), triple_count).expect("|T| > 0");
        e_a.extend(set.eav.iter_prefix::<16>()
            .map(|(e, count)| (id_into_value(e), count as usize))
            .map(|(e, count)| (domain.search(&e).expect("e in domain"), count))
            .flat_map(|(e, count)| iter::repeat(e).take(count))).unwrap();
        let e_a = e_a.build();

        let mut a_a = EliasFanoBuilder::new(domain.len(), triple_count).expect("|T| > 0");
        a_a.extend(set.aev.iter_prefix::<16>()
            .map(|(a, count)| (id_into_value(a), count as usize))
            .map(|(a, count)| (domain.search(&a).expect("a in domain"), count))
            .flat_map(|(a, count)| iter::repeat(a).take(count))).unwrap();
        let a_a = a_a.build();

        let mut v_a = EliasFanoBuilder::new(domain.len(), triple_count).expect("|T| > 0");
        v_a.extend(set.vea.iter_prefix::<32>()
            .map(|(v, count)| (v, count as usize))
            .map(|(v, count)| (domain.search(&v).expect("v in domain"), count))
            .flat_map(|(v, count)| iter::repeat(v).take(count))).unwrap();
        let v_a = v_a.build();

        //eav
        let mut eav_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        eav_c.extend(set.eav.iter_prefix::<64>()
            .map(|(t, _)| t[32..64].try_into().unwrap())
            .map(|v| domain.search(&v).expect("v in domain"))).unwrap();
        let eav_c = WaveletMatrix::new(eav_c).unwrap();

        //vea
        let mut vea_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        vea_c.extend(set.vea.iter_prefix::<64>()
            .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
            .map(|a| domain.search(&a).expect("a in domain"))).unwrap();
        let vea_c = WaveletMatrix::new(vea_c).unwrap();

        //ave
        let mut ave_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        ave_c.extend(set.ave.iter_prefix::<64>()
            .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
            .map(|e| domain.search(&e).expect("e in domain"))).unwrap();
        let ave_c = WaveletMatrix::new(ave_c).unwrap();

        //vae
        let mut vae_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        vae_c.extend(set.vae.iter_prefix::<64>()
            .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
            .map(|e| domain.search(&e).expect("e in domain"))).unwrap();
        let vae_c = WaveletMatrix::new(vae_c).unwrap();

        //eva
        let mut eva_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        eva_c.extend(set.eva.iter_prefix::<64>()
            .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
            .map(|a| domain.search(&a).expect("a in domain"))).unwrap();
        let eva_c = WaveletMatrix::new(eva_c).unwrap();

        //aev
        let mut aev_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        aev_c.extend(set.aev.iter_prefix::<64>()
            .map(|(t, _)| t[32..64].try_into().unwrap())
            .map(|v| domain.search(&v).expect("v in domain"))).unwrap();
        let aev_c = WaveletMatrix::new(aev_c).unwrap();

        TribleSetArchive {
            domain,
            e_a,
            a_a,
            v_a,
            eav_c,
            vea_c,
            ave_c,
            vae_c,
            eva_c,
            aev_c,
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
*/

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use crate::{trible::Trible, ufoid, NS};

    use super::*;
    use fake::{faker::name::raw::Name, locales::EN, Fake};
    use itertools::Itertools;
    use proptest::prelude::*;
    use sucds::bit_vectors::Rank9Sel;

    NS! {
        pub namespace knights {
            "328edd7583de04e2bedd6bd4fd50e651" as loves: crate::Id;
            "328147856cc1984f0806dbb824d2b4cb" as name: crate::types::SmallString;
        }
    }

    proptest! {
        #[test]
        fn create(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut set = TribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.insert(&Trible{ data: key});
            }

            let _archive = TribleSetArchive::<CompressedUniverse<DacsOpt>, Rank9Sel>::with(&set);
        }

        #[test]
        fn ordered_universe(values in prop::collection::vec(prop::collection::vec(0u8..255, 32), 1..10000)) {
            let mut values: Vec<Value> = values.into_iter().map(|v| v.try_into().unwrap()).collect();
            values.sort();
            let u = OrderedUniverse::with(values.iter().copied());
            for i in 0..u.len() {
                let original = values[i];
                let reconstructed = u.access(i);
                assert_eq!(original, reconstructed);
            }
            for i in 0..u.len() {
                let original = Some(i);
                let found = u.search(&values[i]);
                assert_eq!(original, found);
            }
        }

        #[test]
        fn compressed_universe(values in prop::collection::vec(prop::collection::vec(0u8..255, 32), 1..10000)) {
            let mut values: Vec<Value> = values.into_iter().map(|v| v.try_into().unwrap()).collect();
            values.sort();
            let u = CompressedUniverse::<DacsOpt>::with(values.iter().copied());
            for i in 0..u.len() {
                let original = values[i];
                let reconstructed = u.access(i);
                assert_eq!(original, reconstructed);
            }
            for i in 0..u.len() {
                let original = Some(i);
                let found = u.search(&values[i]);
                assert_eq!(original, found);
            }
        }
    }
}
