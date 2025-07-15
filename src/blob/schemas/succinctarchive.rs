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

use sucds::int_vectors::CompactVector;

fn build_prefix_bv<B, I>(domain_len: usize, triple_count: usize, iter: I) -> B
where
    B: Build + Access + Rank + Select + NumBits,
    I: IntoIterator<Item = (usize, usize)>,
{
    let mut bits = vec![false; triple_count + domain_len + 1];
    let mut seen = 0usize;
    let mut last = 0usize;
    for (val, count) in iter {
        for c in last..=val {
            bits[seen + c] = true;
        }
        seen += count;
        last = val + 1;
    }
    for c in last..=domain_len {
        bits[seen + c] = true;
    }
    B::build_from_bits(bits.into_iter(), true, true, true).unwrap()
}

#[derive(Debug, Clone)]
pub struct SuccinctArchive<U, B> {
    pub domain: U,

    pub entity_count: usize,
    pub attribute_count: usize,
    pub value_count: usize,

    pub e_a: B,
    pub a_a: B,
    pub v_a: B,

    /// Bit vector marking the first occurrence of each `(entity, attribute)` pair
    /// in `eav_c`.
    pub changed_e_a: B,
    /// Bit vector marking the first occurrence of each `(entity, value)` pair in
    /// `eva_c`.
    pub changed_e_v: B,
    /// Bit vector marking the first occurrence of each `(attribute, entity)` pair
    /// in `aev_c`.
    pub changed_a_e: B,
    /// Bit vector marking the first occurrence of each `(attribute, value)` pair
    /// in `ave_c`.
    pub changed_a_v: B,
    /// Bit vector marking the first occurrence of each `(value, entity)` pair in
    /// `vea_c`.
    pub changed_v_e: B,
    /// Bit vector marking the first occurrence of each `(value, attribute)` pair
    /// in `vae_c`.
    pub changed_v_a: B,

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
            let a_i = self.v_a.select1(v).unwrap() - v + self.eav_c.rank(v_i, v).unwrap();
            let a = self.vea_c.access(a_i).unwrap();
            let e_i = self.a_a.select1(a).unwrap() - a + self.vea_c.rank(a_i, a).unwrap();
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

    /// Count the number of set bits in `bv` within `range`.
    ///
    /// The bit vectors in this archive encode the first occurrence of each
    /// component pair.  By counting the set bits between two offsets we can
    /// quickly determine how many distinct pairs appear in that slice of the
    /// index.
    pub fn distinct_in(&self, bv: &B, range: &std::ops::Range<usize>) -> usize {
        bv.rank1(range.end).unwrap() - bv.rank1(range.start).unwrap()
    }

    /// Enumerate the rotated offsets of set bits in `bv` within `range`.
    ///
    /// `bv` marks the first occurrence of component pairs in the ordering that
    /// produced `col`.  For each selected bit this function reads the component
    /// value from `col` and uses `prefix` to translate the index to the adjacent
    /// orientation.  The iterator therefore yields indices positioned to access
    /// the middle component of each pair.
    pub fn enumerate_in<'a>(
        &'a self,
        bv: &'a B,
        range: &std::ops::Range<usize>,
        col: &'a WaveletMatrix<B>,
        prefix: &'a B,
    ) -> impl Iterator<Item = usize> + 'a {
        let start = bv.rank1(range.start).unwrap();
        let end = bv.rank1(range.end).unwrap();
        (start..end).map(move |r| {
            let idx = bv.select1(r).unwrap();
            let val = col.access(idx).unwrap();
            prefix.select1(val).unwrap() - val + col.rank(idx, val).unwrap()
        })
    }

    /// Enumerate the identifiers present in `prefix` using `rank`/`select` to
    /// jump directly to the next distinct prefix sum.
    pub fn enumerate_domain<'a>(&'a self, prefix: &'a B) -> impl Iterator<Item = RawValue> + 'a {
        let zero_count = prefix.num_bits() - (self.domain.len() + 1);
        let mut z = 0usize;
        std::iter::from_fn(move || {
            if z >= zero_count {
                return None;
            }
            let pos = prefix.select0(z).unwrap();
            let id = prefix.rank1(pos).unwrap() - 1;
            z = prefix.rank0(prefix.select1(id + 1).unwrap()).unwrap();
            Some(self.domain.access(id))
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

        let entity_count = set.eav.segmented_len(&[0; 0]) as usize;
        let attribute_count = set.ave.segmented_len(&[0; 0]) as usize;
        let value_count = set.vea.segmented_len(&[0; 0]) as usize;

        let e_iter = set
            .eav
            .iter_prefix_count::<16>()
            .map(|(e, _)| id_into_value(&e));
        let a_iter = set
            .ave
            .iter_prefix_count::<16>()
            .map(|(a, _)| id_into_value(&a));
        let v_iter = set.vea.iter_prefix_count::<32>().map(|(v, _)| v);

        let domain = U::with(e_iter.merge(a_iter).merge(v_iter).dedup());
        let alph_width = sucds::utils::needed_bits(domain.len() - 1);

        let e_a = build_prefix_bv(
            domain.len(),
            triple_count,
            set.eav.iter_prefix_count::<16>().map(|(e, c)| {
                (
                    domain.search(&id_into_value(&e)).expect("e in domain"),
                    c as usize,
                )
            }),
        );

        let a_a = build_prefix_bv(
            domain.len(),
            triple_count,
            set.ave.iter_prefix_count::<16>().map(|(a, c)| {
                (
                    domain.search(&id_into_value(&a)).expect("a in domain"),
                    c as usize,
                )
            }),
        );

        let v_a = build_prefix_bv(
            domain.len(),
            triple_count,
            set.vea
                .iter_prefix_count::<32>()
                .map(|(v, c)| (domain.search(&v).expect("v in domain"), c as usize)),
        );

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

        // Build bit vectors marking the first occurrence of each pair
        let changed_e_a = B::build_from_bits(
            set.eav.iter_prefix_count::<32>().flat_map(|(_, c)| {
                iter::once(true).chain(iter::repeat(false).take(c as usize - 1))
            }),
            true,
            true,
            true,
        )
        .unwrap();

        let changed_e_v = B::build_from_bits(
            set.eva.iter_prefix_count::<48>().flat_map(|(_, c)| {
                iter::once(true).chain(iter::repeat(false).take(c as usize - 1))
            }),
            true,
            true,
            true,
        )
        .unwrap();

        let changed_a_e = B::build_from_bits(
            set.aev.iter_prefix_count::<32>().flat_map(|(_, c)| {
                iter::once(true).chain(iter::repeat(false).take(c as usize - 1))
            }),
            true,
            true,
            true,
        )
        .unwrap();

        let changed_a_v = B::build_from_bits(
            set.ave.iter_prefix_count::<48>().flat_map(|(_, c)| {
                iter::once(true).chain(iter::repeat(false).take(c as usize - 1))
            }),
            true,
            true,
            true,
        )
        .unwrap();

        let changed_v_e = B::build_from_bits(
            set.vea.iter_prefix_count::<48>().flat_map(|(_, c)| {
                iter::once(true).chain(iter::repeat(false).take(c as usize - 1))
            }),
            true,
            true,
            true,
        )
        .unwrap();

        let changed_v_a = B::build_from_bits(
            set.vae.iter_prefix_count::<48>().flat_map(|(_, c)| {
                iter::once(true).chain(iter::repeat(false).take(c as usize - 1))
            }),
            true,
            true,
            true,
        )
        .unwrap();

        SuccinctArchive {
            domain,
            entity_count,
            attribute_count,
            value_count,
            e_a,
            a_a,
            v_a,
            changed_e_a,
            changed_e_v,
            changed_a_e,
            changed_a_v,
            changed_v_e,
            changed_v_a,
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
        fn create(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..128)) {
            let mut set = TribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.insert(&Trible{ data: key});
            }

            let _archive: SuccinctArchive::<CompressedUniverse<DacsOpt>, Rank9Sel> = (&set).into();
        }

        #[test]
        fn roundtrip(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..128)) {
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
        fn ordered_universe(values in prop::collection::vec(prop::collection::vec(0u8..255, 32), 1..128)) {
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
        fn compressed_universe(values in prop::collection::vec(prop::collection::vec(0u8..255, 32), 1..128)) {
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
