use crate::Value;
use crate::VALUE_LEN;

use std::convert::TryInto;
use std::ops::Range;
use std::ops::RangeBounds;

use indxvec::Search;
use sucds::int_vectors::{ Build as IBuild, Access as IAccess, NumVals};
use sucds::Serializable;

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
