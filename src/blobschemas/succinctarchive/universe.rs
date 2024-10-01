use crate::RawValue;
use crate::VALUE_LEN;

use std::cmp::Reverse;
use std::collections::HashMap;
use std::convert::TryInto;

use hifitime::Frequencies;
use indxvec::Search;
use sucds::int_vectors::{Access as IAccess, Build as IBuild, NumVals};
use sucds::Serializable;

pub trait Universe {
    fn with<I>(iter: I) -> Self
    where
        I: Iterator<Item = RawValue>;
    fn access(&self, pos: usize) -> RawValue;
    fn search(&self, v: &RawValue) -> Option<usize>;
    fn size_in_bytes(&self) -> usize;
    fn len(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct OrderedUniverse {
    values: Vec<RawValue>,
}

impl Universe for OrderedUniverse {
    fn with<I>(iter: I) -> Self
    where
        I: Iterator<Item = RawValue>,
    {
        Self {
            values: iter.collect(),
        }
    }

    fn access(&self, pos: usize) -> RawValue {
        self.values[pos]
    }

    fn search(&self, v: &RawValue) -> Option<usize> {
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
    fragments: [u8; 256],
    data: C,
}

impl<C> Universe for CompressedUniverse<C>
where
    C: IBuild + IAccess + NumVals + Serializable,
{
    fn with<I>(iter: I) -> Self
    where
        I: Iterator<Item = RawValue>,
    {
        let mut universe: Vec<[u8; 32]> = iter.collect();
        universe.sort();
        let mut data = universe.into_flattened();


        let mut frequency: [u64; 256] = [0; 256];

        for &fragment in data.iter() {
            frequency[fragment as usize] += 1;
        }

        let mut fragments: [u8; 256] = core::array::from_fn(|i| i as u8);
        fragments.sort_by_key(|&fragment| Reverse(frequency[fragment as usize]));
        let fragments = fragments;

        let mut fragment_index: [u8; 256] = [0; 256];
        for i in 0..256 {
            fragment_index[fragments[i] as usize] = i as u8;
        }

        for byte in data.iter_mut() {
            *byte = fragment_index[*byte as usize];
        }

        let data = C::build_from_slice(&data).unwrap();

        Self { data, fragments }
    }

    fn access(&self, pos: usize) -> RawValue {
        let mut v: RawValue = [0; 32];

        for i in 0..32 {
            v[i] = self.fragments[self.data.access((pos * 32) + i).unwrap()];
        }

        v
    }

    fn search(&self, v: &RawValue) -> Option<usize> {
        (0..=self.len() - 1)
            .binary_by(|p| self.access(p).cmp(v))
            .ok()
    }

    fn size_in_bytes(&self) -> usize {
        size_of::<[u8; 256]>() + self.data.size_in_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.num_vals() / 32
    }
}

#[cfg(test)]
mod tests {
    use std::iter::repeat_with;

    use sucds::int_vectors::{DacsByte, DacsOpt};

    use crate::{fucid, genid, id_into_value, ufoid};

    use super::{CompressedUniverse, OrderedUniverse, Universe};

    #[test]
    fn ids_compressed() {
        let size = 1000;

        let count_data: Vec<_> = (0..size as u128)
            .map(|id| id_into_value(&id.to_be_bytes()))
            .collect();
        let genid_data: Vec<_> = repeat_with(|| id_into_value(&genid())).take(size).collect();
        let ufoid_data: Vec<_> = repeat_with(|| id_into_value(&ufoid())).take(size).collect();
        let fucid_data: Vec<_> = repeat_with(|| id_into_value(&fucid())).take(size).collect();

        let count_universe = CompressedUniverse::<DacsOpt>::with(count_data.iter().copied());
        let fucid_universe = CompressedUniverse::<DacsOpt>::with(fucid_data.iter().copied());
        let ufoid_universe = CompressedUniverse::<DacsOpt>::with(ufoid_data.iter().copied());
        let genid_universe = CompressedUniverse::<DacsOpt>::with(genid_data.iter().copied());

        println!(
            "count universe bytes per entry: {}",
            count_universe.size_in_bytes() as f64 / size as f64
        );
        println!(
            "fucid universe bytes per entry: {}",
            fucid_universe.size_in_bytes() as f64 / size as f64
        );
        println!(
            "ufoid universe bytes per entry: {}",
            ufoid_universe.size_in_bytes() as f64 / size as f64
        );
        println!(
            "genid universe bytes per entry: {}",
            genid_universe.size_in_bytes() as f64 / size as f64
        );
    }

    #[test]
    fn ids_uncompressed() {
        let size = 1000;

        let count_data: Vec<_> = (0..size as u128)
            .map(|id| id_into_value(&id.to_be_bytes()))
            .collect();
        let genid_data: Vec<_> = repeat_with(|| id_into_value(&genid())).take(size).collect();
        let ufoid_data: Vec<_> = repeat_with(|| id_into_value(&ufoid())).take(size).collect();
        let fucid_data: Vec<_> = repeat_with(|| id_into_value(&fucid())).take(size).collect();

        let count_universe = OrderedUniverse::with(count_data.iter().copied());
        let fucid_universe = OrderedUniverse::with(fucid_data.iter().copied());
        let ufoid_universe = OrderedUniverse::with(ufoid_data.iter().copied());
        let genid_universe = OrderedUniverse::with(genid_data.iter().copied());

        println!(
            "count universe bytes per entry: {}",
            count_universe.size_in_bytes() as f64 / size as f64
        );
        println!(
            "fucid universe bytes per entry: {}",
            fucid_universe.size_in_bytes() as f64 / size as f64
        );
        println!(
            "ufoid universe bytes per entry: {}",
            ufoid_universe.size_in_bytes() as f64 / size as f64
        );
        println!(
            "genid universe bytes per entry: {}",
            genid_universe.size_in_bytes() as f64 / size as f64
        );
    }
}
