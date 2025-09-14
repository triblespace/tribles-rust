mod succinctarchiveconstraint;
mod universe;

use crate::id::id_from_value;
use crate::id::id_into_value;
use crate::id::Id;
use crate::prelude::*;
use crate::query::TriblePattern;
use crate::trible::Trible;
use crate::trible::TribleSet;
use crate::value::schemas::genid::GenId;
use crate::value::schemas::UnknownValue;
use crate::value::RawValue;
use crate::value::Value;
use crate::value::ValueSchema;
use succinctarchiveconstraint::*;

pub use universe::*;

use std::convert::TryInto;
use std::iter;

use itertools::Itertools;

use jerky::bit_vector::rank9sel::Rank9SelIndex;
use jerky::bit_vector::BitVector;
use jerky::bit_vector::BitVectorBuilder;
use jerky::bit_vector::NumBits;
use jerky::bit_vector::Rank;
use jerky::bit_vector::Select;
use jerky::char_sequences::WaveletMatrix;
use jerky::int_vectors::CompactVector;

fn build_prefix_bv<I>(domain_len: usize, triple_count: usize, iter: I) -> BitVector<Rank9SelIndex>
where
    I: IntoIterator<Item = (usize, usize)>,
{
    let mut builder = BitVectorBuilder::from_bit(false, triple_count + domain_len + 1);

    let mut seen = 0usize;
    let mut last = 0usize;
    for (val, count) in iter {
        for c in last..=val {
            builder.set_bit(seen + c, true).unwrap();
        }
        seen += count;
        last = val + 1;
    }
    for c in last..=domain_len {
        builder.set_bit(seen + c, true).unwrap();
    }
    builder.freeze::<Rank9SelIndex>()
}

#[derive(Debug, Clone)]
pub struct SuccinctArchive<U> {
    pub domain: U,

    pub entity_count: usize,
    pub attribute_count: usize,
    pub value_count: usize,

    pub e_a: BitVector<Rank9SelIndex>,
    pub a_a: BitVector<Rank9SelIndex>,
    pub v_a: BitVector<Rank9SelIndex>,

    /// Bit vector marking the first occurrence of each `(entity, attribute)` pair
    /// in `eav_c`.
    pub changed_e_a: BitVector<Rank9SelIndex>,
    /// Bit vector marking the first occurrence of each `(entity, value)` pair in
    /// `eva_c`.
    pub changed_e_v: BitVector<Rank9SelIndex>,
    /// Bit vector marking the first occurrence of each `(attribute, entity)` pair
    /// in `aev_c`.
    pub changed_a_e: BitVector<Rank9SelIndex>,
    /// Bit vector marking the first occurrence of each `(attribute, value)` pair
    /// in `ave_c`.
    pub changed_a_v: BitVector<Rank9SelIndex>,
    /// Bit vector marking the first occurrence of each `(value, entity)` pair in
    /// `vea_c`.
    pub changed_v_e: BitVector<Rank9SelIndex>,
    /// Bit vector marking the first occurrence of each `(value, attribute)` pair
    /// in `vae_c`.
    pub changed_v_a: BitVector<Rank9SelIndex>,

    pub eav_c: WaveletMatrix<Rank9SelIndex>,
    pub vea_c: WaveletMatrix<Rank9SelIndex>,
    pub ave_c: WaveletMatrix<Rank9SelIndex>,
    pub vae_c: WaveletMatrix<Rank9SelIndex>,
    pub eva_c: WaveletMatrix<Rank9SelIndex>,
    pub aev_c: WaveletMatrix<Rank9SelIndex>,
}

impl<U> SuccinctArchive<U>
where
    U: Universe,
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
    pub fn distinct_in(
        &self,
        bv: &BitVector<Rank9SelIndex>,
        range: &std::ops::Range<usize>,
    ) -> usize {
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
        bv: &'a BitVector<Rank9SelIndex>,
        range: &std::ops::Range<usize>,
        col: &'a WaveletMatrix<Rank9SelIndex>,
        prefix: &'a BitVector<Rank9SelIndex>,
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
    pub fn enumerate_domain<'a>(
        &'a self,
        prefix: &'a BitVector<Rank9SelIndex>,
    ) -> impl Iterator<Item = RawValue> + 'a {
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

impl<U> From<&TribleSet> for SuccinctArchive<U>
where
    U: Universe,
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
        let alph_width = jerky::utils::needed_bits(domain.len() - 1);

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
        let mut eav_b = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        eav_b
            .extend(
                set.eav
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| t[32..64].try_into().unwrap())
                    .map(|v| domain.search(&v).expect("v in domain")),
            )
            .unwrap();
        let eav_c = WaveletMatrix::new(eav_b.freeze()).unwrap();

        //vea
        let mut vea_b = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        vea_b
            .extend(
                set.vea
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
                    .map(|a| domain.search(&a).expect("a in domain")),
            )
            .unwrap();
        let vea_c = WaveletMatrix::new(vea_b.freeze()).unwrap();

        //ave
        let mut ave_b = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        ave_b
            .extend(
                set.ave
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
                    .map(|e| domain.search(&e).expect("e in domain")),
            )
            .unwrap();
        let ave_c = WaveletMatrix::new(ave_b.freeze()).unwrap();

        //vae
        let mut vae_b = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        vae_b
            .extend(
                set.vae
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
                    .map(|e| domain.search(&e).expect("e in domain")),
            )
            .unwrap();
        let vae_c = WaveletMatrix::new(vae_b.freeze()).unwrap();

        //eva
        let mut eva_b = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        eva_b
            .extend(
                set.eva
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| id_into_value(t[48..64].try_into().unwrap()))
                    .map(|a| domain.search(&a).expect("a in domain")),
            )
            .unwrap();
        let eva_c = WaveletMatrix::new(eva_b.freeze()).unwrap();

        //aev
        let mut aev_b = CompactVector::with_capacity(triple_count, alph_width).expect("|D| > 2^32");
        aev_b
            .extend(
                set.aev
                    .iter_prefix_count::<64>()
                    .map(|(t, _)| t[32..64].try_into().unwrap())
                    .map(|v| domain.search(&v).expect("v in domain")),
            )
            .unwrap();
        let aev_c = WaveletMatrix::new(aev_b.freeze()).unwrap();

        // Build bit vectors marking the first occurrence of each pair
        let changed_e_a = {
            let mut b = BitVectorBuilder::new();
            b.extend_bits(set.eav.iter_prefix_count::<32>().flat_map(|(_, c)| {
                iter::once(true).chain(std::iter::repeat_n(false, c as usize - 1))
            }));
            b.freeze::<Rank9SelIndex>()
        };

        let changed_e_v = {
            let mut b = BitVectorBuilder::new();
            b.extend_bits(set.eva.iter_prefix_count::<48>().flat_map(|(_, c)| {
                iter::once(true).chain(std::iter::repeat_n(false, c as usize - 1))
            }));
            b.freeze::<Rank9SelIndex>()
        };

        let changed_a_e = {
            let mut b = BitVectorBuilder::new();
            b.extend_bits(set.aev.iter_prefix_count::<32>().flat_map(|(_, c)| {
                iter::once(true).chain(std::iter::repeat_n(false, c as usize - 1))
            }));
            b.freeze::<Rank9SelIndex>()
        };

        let changed_a_v = {
            let mut b = BitVectorBuilder::new();
            b.extend_bits(set.ave.iter_prefix_count::<48>().flat_map(|(_, c)| {
                iter::once(true).chain(std::iter::repeat_n(false, c as usize - 1))
            }));
            b.freeze::<Rank9SelIndex>()
        };

        let changed_v_e = {
            let mut b = BitVectorBuilder::new();
            b.extend_bits(set.vea.iter_prefix_count::<48>().flat_map(|(_, c)| {
                iter::once(true).chain(std::iter::repeat_n(false, c as usize - 1))
            }));
            b.freeze::<Rank9SelIndex>()
        };

        let changed_v_a = {
            let mut b = BitVectorBuilder::new();
            b.extend_bits(set.vae.iter_prefix_count::<48>().flat_map(|(_, c)| {
                iter::once(true).chain(std::iter::repeat_n(false, c as usize - 1))
            }));
            b.freeze::<Rank9SelIndex>()
        };

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

impl<U> From<&SuccinctArchive<U>> for TribleSet
where
    U: Universe,
{
    fn from(archive: &SuccinctArchive<U>) -> Self {
        archive.iter().collect()
    }
}

impl<U> TriblePattern for SuccinctArchive<U>
where
    U: Universe,
{
    type PatternConstraint<'a>
        = SuccinctArchiveConstraint<'a, U>
    where
        U: 'a;

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
    use crate::prelude::*;
    use crate::query::find;
    use crate::trible::Trible;
    use crate::value::schemas::shortstring::ShortString;
    use crate::value::ToValue;
    use crate::value::TryToValue;

    use super::*;
    use itertools::Itertools;
    use jerky::int_vectors::DacsByte;
    use proptest::prelude::*;

    pub mod knights {
        use crate::prelude::*;

        attributes! {
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

            let _archive: SuccinctArchive<CompressedUniverse<DacsByte>> = (&set).into();
        }

        #[test]
        fn roundtrip(entries in prop::collection::vec(prop::collection::vec(0u8..255, 64), 1..128)) {
            let mut set = TribleSet::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                set.insert(&Trible{ data: key});
            }

            let archive: SuccinctArchive<CompressedUniverse<DacsByte>> = (&set).into();
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
            let u = CompressedUniverse::<DacsByte>::with(values.iter().copied());
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

        kb += entity! { &juliet @
           knights::name: "Juliet",
           knights::loves: &romeo,
           knights::title: "Maiden"
        };
        kb += entity! { &romeo @
           knights::name: "Romeo",
           knights::loves: &juliet,
           knights::title: "Prince"
        };
        kb += entity! {
           knights::name: "Angelica",
           knights::title: "Nurse"
        };

        let archive: SuccinctArchive<OrderedUniverse> = (&kb).into();

        let r: Vec<_> = find!(
            (juliet, name),
            pattern!(&archive, [
            {knights::name: ("Romeo"),
             knights::loves: juliet},
            {juliet @
                knights::name: name
            }])
        )
        .collect();
        assert_eq!(
            vec![((&juliet).to_value(), "Juliet".try_to_value().unwrap(),)],
            r
        );
    }
}
