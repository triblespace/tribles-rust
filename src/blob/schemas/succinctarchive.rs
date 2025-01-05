mod succinctarchiveconstraint;
mod universe;

use crate::id::{id_from_value, id_into_value, Id};
use crate::query::TriblePattern;
use crate::trible::{Trible, TribleSet};
use crate::value::schemas::UnknownValue;
use crate::value::Value;
use crate::value::{schemas::genid::GenId, RawValue, ValueSchema};
use succinctarchiveconstraint::*;

pub use universe::*;

use std::convert::TryInto;
use std::iter;

use itertools::Itertools;

use sucds::bit_vectors::{Access, Build, NumBits, Rank, Select};
use sucds::char_sequences::WaveletMatrix;
use sucds::mii_sequences::{EliasFano, EliasFanoBuilder};

use sucds::int_vectors::CompactVector;

#[derive(Debug, Clone)]
pub struct SuccinctArchive<U, B> {
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

impl<U, B> SuccinctArchive<U, B>
where
    U: Universe,
    B: Build + Access + Rank + Select + NumBits,
{
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = Trible> + 'a {
        (0..self.eav_c.len()).map(move |v_i| {
            let v = self.eav_c.access(v_i).unwrap();
            let a_i = self.v_a.select(v).unwrap() + self.eav_c.rank(v_i, v).unwrap();
            let a = self.vea_c.access(a_i).unwrap();
            let e_i = self.a_a.select(a).unwrap() + self.vea_c.rank(a_i, a).unwrap();
            let e = self.ave_c.access(e_i).unwrap();

            let e = self.domain.access(e);
            let a = self.domain.access(a);
            let v = self.domain.access(v);

            let e: Id = Id::new(id_from_value(&e).unwrap()).unwrap();
            let a: Id = Id::new(id_from_value(&a).unwrap()).unwrap();
            let v: Value<UnknownValue> = Value::new(v);

            Trible::force(&e, &a, &v)
        })
    }
}

impl<U, B> From<&TribleSet> for SuccinctArchive<U, B>
where
    U: Universe,
    B: Build + Access + Rank + Select + NumBits,
{
    fn from(set: &TribleSet) -> Self {
        let triple_count = set.eav.len() as usize;
        assert!(triple_count > 0);

        let e_iter = set.eav.iter_prefix_count::<16>().map(|(e, _)| id_into_value(&e));
        let a_iter = set.ave.iter_prefix_count::<16>().map(|(a, _)| id_into_value(&a));
        let v_iter = set.vea.iter_prefix_count::<32>().map(|(v, _)| v);

        let domain = U::with(e_iter.merge(a_iter).merge(v_iter).dedup());
        let alph_width = sucds::utils::needed_bits(domain.len() - 1);

        let mut e_a = EliasFanoBuilder::new(triple_count + 1, domain.len()).expect("|D| > 0");
        let mut sum = 0;
        let mut last = 0;
        for (e, count) in set
            .eav
            .iter_prefix_count::<16>()
            .map(|(e, count)| (id_into_value(&e), count as usize))
            .map(|(e, count)| (domain.search(&e).expect("e in domain"), count))
        {
            e_a.extend(iter::repeat(sum).take((e + 1) - last)).unwrap();
            sum = sum + count;
            last = e + 1;
        }
        e_a.extend(iter::repeat(sum).take(domain.len() - last))
            .unwrap();
        let e_a = e_a.build();

        let mut a_a = EliasFanoBuilder::new(triple_count + 1, domain.len()).expect("|D| > 0");
        let mut sum = 0;
        let mut last = 0;
        for (a, count) in set
            .aev
            .iter_prefix_count::<16>()
            .map(|(a, count)| (id_into_value(&a), count as usize))
            .map(|(a, count)| (domain.search(&a).expect("a in domain"), count))
        {
            a_a.extend(iter::repeat(sum).take((a + 1) - last)).unwrap();
            sum = sum + count;
            last = a + 1;
        }
        a_a.extend(iter::repeat(sum).take(domain.len() - last))
            .unwrap();
        let a_a = a_a.build();

        let mut v_a = EliasFanoBuilder::new(triple_count + 1, domain.len()).expect("|D| > 0");
        let mut sum = 0;
        let mut last = 0;
        for (v, count) in set
            .vea
            .iter_prefix_count::<32>()
            .map(|(v, count)| (v, count as usize))
            .map(|(v, count)| (domain.search(&v).expect("v in domain"), count))
        {
            v_a.extend(iter::repeat(sum).take((v + 1) - last)).unwrap();
            sum = sum + count;
            last = v + 1;
        }
        v_a.extend(iter::repeat(sum).take(domain.len() - last))
            .unwrap();
        let v_a = v_a.build();

        //eav
        let mut eav_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        eav_c
            .extend(
                set.eav
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| t[32..64].try_into().unwrap())
                    .map(|v| domain.search(&v).expect("v in domain")),
            )
            .unwrap();
        let eav_c = WaveletMatrix::new(eav_c).unwrap();

        //vea
        let mut vea_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        vea_c
            .extend(
                set.vea
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
                    .map(|a| domain.search(&a).expect("a in domain")),
            )
            .unwrap();
        let vea_c = WaveletMatrix::new(vea_c).unwrap();

        //ave
        let mut ave_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        ave_c
            .extend(
                set.ave
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
                    .map(|e| domain.search(&e).expect("e in domain")),
            )
            .unwrap();
        let ave_c = WaveletMatrix::new(ave_c).unwrap();

        //vae
        let mut vae_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        vae_c
            .extend(
                set.vae
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
                    .map(|e| domain.search(&e).expect("e in domain")),
            )
            .unwrap();
        let vae_c = WaveletMatrix::new(vae_c).unwrap();

        //eva
        let mut eva_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        eva_c
            .extend(
                set.eva
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
                    .map(|a| domain.search(&a).expect("a in domain")),
            )
            .unwrap();
        let eva_c = WaveletMatrix::new(eva_c).unwrap();

        //aev
        let mut aev_c = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        aev_c
            .extend(
                set.aev
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| t[32..64].try_into().unwrap())
                    .map(|v| domain.search(&v).expect("v in domain")),
            )
            .unwrap();
        let aev_c = WaveletMatrix::new(aev_c).unwrap();

        SuccinctArchive {
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

impl<U, B> From<&SuccinctArchive<U, B>> for TribleSet
where
    U: Universe,
    B: Build + Access + Rank + Select + NumBits,
{
    fn from(archive: &SuccinctArchive<U, B>) -> Self {
        archive.iter().collect()
    }
}

impl<U, B> TriblePattern for SuccinctArchive<U, B>
where
    U: Universe,
    B: Build + Access + Rank + Select + NumBits,
{
    type PatternConstraint<'a>
        = SuccinctArchiveConstraint<'a, U, B>
    where
        U: 'a,
        B: 'a;

    fn pattern<'a, V: ValueSchema>(
        &'a self,
        e: crate::query::Variable<GenId>,
        a: crate::query::Variable<GenId>,
        v: crate::query::Variable<V>,
    ) -> Self::PatternConstraint<'a> {
        SuccinctArchiveConstraint::new(e, a, v, self)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use crate::id::fucid;
    use crate::namespace::NS;
    use crate::query::find;
    use crate::trible::Trible;
    use crate::value::ToValue;
    use crate::value::{schemas::shortstring::ShortString, TryToValue};

    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;
    use sucds::bit_vectors::Rank9Sel;
    use sucds::int_vectors::DacsOpt;

    NS! {
        pub namespace knights1 {
            "328edd7583de04e2bedd6bd4fd50e651" as loves: GenId;
            "328147856cc1984f0806dbb824d2b4cb" as name: ShortString;
            "328f2c33d2fdd675e733388770b2d6c4" as title: ShortString;
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

            let _archive: SuccinctArchive::<CompressedUniverse<DacsOpt>, Rank9Sel> = (&set).into();
        }

        #[test]
        fn roundtrip(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..1024)) {
            let mut set = TribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.insert(&Trible{ data: key});
            }

            let archive: SuccinctArchive::<CompressedUniverse<DacsOpt>, Rank9Sel> = (&set).into();
            let set_: TribleSet = (&archive).into();

            assert_eq!(set, set_);
        }

        #[test]
        fn ordered_universe(values in prop::collection::vec(prop::collection::vec(0u8..255, 32), 1..1024)) {
            let mut values: Vec<RawValue> = values.into_iter().map(|v| v.try_into().unwrap()).collect();
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
        fn compressed_universe(values in prop::collection::vec(prop::collection::vec(0u8..255, 32), 1..1024)) {
            let mut values: Vec<RawValue> = values.into_iter().map(|v| v.try_into().unwrap()).collect();
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

    #[test]
    fn archive_pattern() {
        let juliet = fucid();
        let romeo = fucid();

        let mut kb = TribleSet::new();

        kb += knights1::entity!(&juliet,
        {
            name: "Juliet",
            loves: &romeo,
            title: "Maiden"
        });
        kb += knights1::entity!(&romeo, {
            name: "Romeo",
            loves: &juliet,
            title: "Prince"
        });
        kb += knights1::entity!({
            name: "Angelica",
            title: "Nurse"
        });

        let archive: SuccinctArchive<OrderedUniverse, Rank9Sel> = (&kb).into();

        let r: Vec<_> = find!(
            (juliet, name),
            knights1::pattern!(&archive, [
            {name: ("Romeo"),
             loves: juliet},
            {juliet @
                name: name
            }])
        )
        .collect();
        assert_eq!(
            vec![((&juliet).to_value(), "Juliet".try_to_value().unwrap(),)],
            r
        );
    }
}
