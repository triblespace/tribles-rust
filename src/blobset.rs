use digest::{typenum::U32, Digest};

use crate::blobschemas::UnknownBlob;
use crate::valueschemas::Hash;
use crate::{valueschemas::Handle, TribleSet};
use crate::{Blob, BlobSchema, Value};
use std::collections::HashMap;
use std::iter::FromIterator;

/// A mapping from [Handle]s to [Blob]s.
#[derive(Debug, Clone)]
pub struct BlobSet<H> {
    blobs: HashMap<Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>>,
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

    pub fn insert<T>(&mut self, blob: Blob<T>) -> Value<Handle<H, T>>
    where T: BlobSchema {
        let handle = blob.as_handle();
        let unknown_handle: Value<Handle<H, UnknownBlob>> = Value::new(handle.bytes);
        let blob: Blob<UnknownBlob> = Blob::new(blob.bytes);
        self.blobs.insert(unknown_handle, blob);
        handle
    }

    pub fn get<T>(&self, handle: Value<Handle<H, T>>) -> Option<Blob<T>>
    where
        T: BlobSchema,
    {
        let hash: Value<Hash<_>> = handle.into();
        let handle: Value<Handle<H, UnknownBlob>> = hash.into();
        let Some(blob) = self.blobs.get(&handle) else {
            return None;
        };
        Some(Blob::new(blob.bytes.clone()))
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a Value<Handle<H, UnknownBlob>>, &'a Blob<UnknownBlob>)> + 'a {
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

impl<H> FromIterator<(Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>)> for BlobSet<H>
where
    H: Digest<OutputSize = U32>,
{
    fn from_iter<I: IntoIterator<Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>)>>(iter: I) -> Self {
        let mut set = BlobSet::new();

        for (handle, blob) in iter {
            set.blobs.insert(handle, blob);
        }

        set
    }
}

impl<'a, H> IntoIterator for BlobSet<H>
where
    H: Digest<OutputSize = U32>,
{
    type Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>);
    type IntoIter = std::collections::hash_map::IntoIter<Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>>;

    fn into_iter(self) -> Self::IntoIter {
        self.blobs.into_iter()
    }
}

impl<'a, H> IntoIterator for &'a BlobSet<H>
where
    H: Digest<OutputSize = U32>,
{
    type Item = (&'a Value<Handle<H, UnknownBlob>>, &'a Blob<UnknownBlob>);
    type IntoIter = std::collections::hash_map::Iter<'a, Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.blobs).into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        blobschemas::{PackBlob, ZCString}, valueschemas::{hash::Blake3, Handle}, TribleSet, NS
    };

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
            blobs_a.insert(ZCString::from(Name(EN).fake::<String>()).pack());
        }
        for _i in 0..1000 {
            blobs_b.insert(ZCString::from(Name(EN).fake::<String>()).pack());
        }

        blobs_a.union(blobs_b);
    }

    #[test]
    fn keep() {
        let mut kb = TribleSet::new();
        let mut blobs = BlobSet::new();
        for _i in 0..2000 {
            kb.union(knights::entity!({
                description: blobs.insert(ZCString::from(Name(EN).fake::<String>()).pack())
            }));
        }
        blobs.keep(kb);
    }
}
