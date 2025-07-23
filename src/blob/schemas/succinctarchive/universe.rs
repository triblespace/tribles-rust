use crate::value::{RawValue, VALUE_LEN};

use std::cmp::Reverse;
use std::collections::HashMap;
use std::convert::{Infallible, TryInto};

use indxvec::Search;
use jerky::int_vectors::{Access as IAccess, Build as IBuild, DacsByte, NumVals};
use quick_cache::sync::Cache;

pub trait Universe {
    fn with<I>(iter: I) -> Self
    where
        I: Iterator<Item = RawValue>;
    fn access(&self, pos: usize) -> RawValue;
    fn search(&self, v: &RawValue) -> Option<usize>;
    fn size_in_bytes(&self) -> usize;
    fn len(&self) -> usize;
}

pub trait SizeInBytes {
    fn size_in_bytes(&self) -> usize;
}

impl SizeInBytes for DacsByte {
    fn size_in_bytes(&self) -> usize {
        DacsByte::size_in_bytes(self)
    }
}

impl SizeInBytes for jerky::int_vectors::CompactVector {
    fn size_in_bytes(&self) -> usize {
        self.size_in_bytes()
    }
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
    fragments: Vec<[u8; 4]>,
    data: C,
}

impl<C> Universe for CompressedUniverse<C>
where
    C: IBuild + IAccess + NumVals + SizeInBytes,
{
    fn with<I>(iter: I) -> Self
    where
        I: Iterator<Item = RawValue>,
    {
        let mut universe: Vec<[u8; 32]> = iter.collect();
        universe.sort();
        let universe = universe;

        let mut data: Vec<[u8; 4]> = Vec::new();
        let mut frequency: HashMap<[u8; 4], u64> = HashMap::new();

        for value in universe {
            for i in 0..8 {
                let fragment = value[i * 4..i * 4 + 4].try_into().unwrap();
                *frequency.entry(fragment).or_insert(0) += 1;
                data.push(fragment);
            }
        }

        let mut fragments: Vec<_> = frequency.keys().copied().collect();
        fragments.sort_by_key(|fragment| (Reverse(frequency.get(fragment)), *fragment));
        let fragments = fragments;

        let fragment_index: HashMap<[u8; 4], u32> = fragments
            .iter()
            .enumerate()
            .map(|(pos, value)| (*value, pos as u32))
            .collect();

        let data: Vec<u32> = data
            .into_iter()
            .map(|fragment| {
                *fragment_index
                    .get(&fragment)
                    .expect("fragment in fragments")
            })
            .collect();

        let data = C::build_from_slice(&data).unwrap();

        Self { data, fragments }
    }

    fn access(&self, pos: usize) -> RawValue {
        let mut v: RawValue = [0; 32];

        for i in 0..8 {
            v[i * 4..i * 4 + 4]
                .copy_from_slice(&(self.fragments[self.data.access((pos * 8) + i).unwrap()]));
        }

        v
    }

    fn search(&self, v: &RawValue) -> Option<usize> {
        (0..=self.len() - 1)
            .binary_by(|p| self.access(p).cmp(v))
            .ok()
    }

    fn size_in_bytes(&self) -> usize {
        self.fragments.len() * size_of::<[u8; 4]>() + self.data.size_in_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.num_vals() / 8
    }
}

#[derive(Debug)]
pub struct CachedUniverse<const ACCESS_CACHE: usize, const SEARCH_CACHE: usize, U: Universe> {
    access_cache: Cache<usize, RawValue>,
    search_cache: Cache<RawValue, Option<usize>>,
    inner: U,
}

impl<const ACCESS_CACHE: usize, const SEARCH_CACHE: usize, U> Universe
    for CachedUniverse<ACCESS_CACHE, SEARCH_CACHE, U>
where
    U: Universe,
{
    fn with<I>(iter: I) -> Self
    where
        I: Iterator<Item = RawValue>,
    {
        Self {
            access_cache: Cache::new(ACCESS_CACHE),
            search_cache: Cache::new(SEARCH_CACHE),
            inner: U::with(iter),
        }
    }

    fn access(&self, pos: usize) -> RawValue {
        self.access_cache
            .get_or_insert_with::<_, Infallible>(&pos, || Ok(self.inner.access(pos)))
            .unwrap()
    }

    fn search(&self, v: &RawValue) -> Option<usize> {
        self.search_cache
            .get_or_insert_with::<_, Infallible>(v, || {
                Ok((0..=self.len() - 1)
                    .binary_by(|p| self.access(p).cmp(v))
                    .ok())
            })
            .unwrap()
    }

    fn size_in_bytes(&self) -> usize {
        self.inner.size_in_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {
    use std::iter::repeat_with;

    use jerky::int_vectors::DacsByte as DacsOpt;

    use crate::id::{fucid, id_into_value, rngid, ufoid};

    use super::{CompressedUniverse, OrderedUniverse, Universe};

    #[test]
    fn ids_compressed() {
        let size = 100;

        let count_data: Vec<_> = (0..size as u128)
            .map(|id| id_into_value(&id.to_be_bytes()))
            .collect();
        let genid_data: Vec<_> = repeat_with(|| id_into_value(&rngid())).take(size).collect();
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
        let size = 100;

        let count_data: Vec<_> = (0..size as u128)
            .map(|id| id_into_value(&id.to_be_bytes()))
            .collect();
        let genid_data: Vec<_> = repeat_with(|| id_into_value(&rngid())).take(size).collect();
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
