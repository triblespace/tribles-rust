use digest::{ Digest, typenum::U32 };
use anybytes::Bytes;

use crate::schemas::Hash;
use crate::{BlobParseError, Bloblike, Value};
use crate::{TribleSet, schemas::Handle};
use std::collections::HashMap;
use std::iter::FromIterator;

/// A mapping from [Handle]s to [Blob]s.
#[derive(Debug, Clone)]
pub struct BlobSet<H> {
    blobs: HashMap<Value<Hash<H>>, Bytes>,
}

impl<H> Eq for BlobSet<H> {}

impl<H> PartialEq for BlobSet<H> {
    fn eq(&self, other: &Self) -> bool {
        self.blobs == other.blobs
    }
}

impl<H> BlobSet<H>
where
    H: Digest<OutputSize = U32>,
{
    pub fn union<'a>(&mut self, other: Self) {
        self.blobs.extend(other);
    }

    pub fn new() -> BlobSet<H> {
        BlobSet {
            blobs: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    pub fn insert<T>(&mut self, value: T) -> Value<Handle<H, T>>
    where
        T: Bloblike,
    {
        let blob: Bytes = value.into_blob();
        let hash = self.insert_raw(blob);
        Value::new(hash.bytes)
    }

    pub fn get<'a, T>(&'a self, handle: Value<Handle<H, T>>) -> Option<Result<T, BlobParseError>>
    where
        T: Bloblike,
    {
        let blob = self.get_raw(handle.into())?;
        Some(T::from_blob(blob.clone()))
    }

    pub fn get_raw(&self, hash: Value<Hash<H>>) -> Option<&Bytes> {
        self.blobs.get(&hash)
    }

    pub fn insert_raw(&mut self, blob: Bytes) -> Value<Hash<H>> {
        let hash = Hash::digest(&blob);
        self.blobs.insert(hash, blob);
        hash
    }

    pub fn iter_raw<'a>(&'a self) -> impl Iterator<Item = (&'a Value<Hash<H>>, &'a Bytes)> + 'a {
        self.blobs.iter()
    }

    // Note that keep is conservative and keeps every blob for which there exists
    // a corresponding trible value, irrespective of that tribles attribute type.
    // This could theoretically allow an attacker to DOS blob garbage collection
    // by introducting values that look like existing hashes, but are actually of
    // a different type. But this is under the assumption that an attacker is only
    // allowed to write non-handle typed triples, otherwise they might as well
    // introduce blobs directly.
    pub fn keep(&mut self, tribles: TribleSet) {
        self.blobs.retain(|k, _| tribles.vae.has_prefix(&k.bytes));
    }
}

impl<H> FromIterator<(Value<Hash<H>>, Bytes)> for BlobSet<H>
where
    H: Digest<OutputSize = U32>,
{
    fn from_iter<I: IntoIterator<Item = (Value<Hash<H>>, Bytes)>>(iter: I) -> Self {
        let mut set = BlobSet::new();

        for (hash, blob) in iter {
            set.blobs.insert(hash, blob);
        }

        set
    }
}

impl<'a, H> IntoIterator for BlobSet<H>
where
    H: Digest<OutputSize = U32>,
{
    type Item = (Value<Hash<H>>, Bytes);
    type IntoIter = std::collections::hash_map::IntoIter<Value<Hash<H>>, Bytes>;

    fn into_iter(self) -> Self::IntoIter {
        self.blobs.into_iter()
    }
}

impl<'a, H> IntoIterator for &'a BlobSet<H>
where
    H: Digest<OutputSize = U32>,
{
    type Item = (&'a Value<Hash<H>>, &'a Bytes);
    type IntoIter = std::collections::hash_map::Iter<'a, Value<Hash<H>>, Bytes>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.blobs).into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::{schemas::{Handle, hash::Blake3, ZCString }, TribleSet, NS};

    use super::*;
    use fake::{faker::name::raw::Name, locales::EN, Fake};

    NS! {
        pub namespace knights {
            "5AD0FAFB1FECBC197A385EC20166899E" as description: Handle<Blake3, ZCString>;
        }
    }

    #[test]
    fn union() {
        let mut blobs_a: BlobSet<Blake3> = BlobSet::new();
        let mut blobs_b: BlobSet<Blake3> = BlobSet::new();

        for _i in 0..1000 {
            blobs_a.insert(ZCString::from(Name(EN).fake::<String>()));
        }
        for _i in 0..1000 {
            blobs_b.insert(ZCString::from(Name(EN).fake::<String>()));
        }

        blobs_a.union(blobs_b);
    }

    #[test]
    fn keep() {
        let mut kb = TribleSet::new();
        let mut blobs = BlobSet::new();
        for _i in 0..2000 {
            kb.union(knights::entity!({
                description: blobs.insert(Name(EN).fake::<String>().into())
            }));
        }
        blobs.keep(kb);
    }
}
