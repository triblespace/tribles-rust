use crate::blob::{schemas::UnknownBlob, Blob, BlobSchema};
use crate::blob::{FromBlob, ToBlob};
use crate::repo::{BlobStorage, BlobStoreGetOp, BlobStoreListOp, BlobStorePutOp};
use crate::trible::TribleSet;
use crate::value::schemas::hash::{Handle, Hash, HashProtocol};
use crate::value::Value;

use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::iter::FromIterator;

/// A mapping from [Handle]s to [Blob]s.
#[derive(Debug, Clone)]
pub struct BlobSet<H: HashProtocol> {
    blobs: HashMap<Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>>,
}

impl<H: HashProtocol> Eq for BlobSet<H> {}

impl<H: HashProtocol> PartialEq for BlobSet<H> {
    fn eq(&self, other: &Self) -> bool {
        self.blobs == other.blobs
    }
}

impl<H: HashProtocol> BlobSet<H> {
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

    pub fn insert<T, S>(&mut self, item: T) -> Value<Handle<H, S>>
    where
        S: BlobSchema,
        T: ToBlob<S>,
    {
        let blob: Blob<S> = ToBlob::to_blob(item);
        let handle = blob.get_handle();
        let unknown_handle: Value<Handle<H, UnknownBlob>> = handle.transmute();
        let blob: Blob<UnknownBlob> = Blob::new(blob.bytes);
        self.blobs.insert(unknown_handle, blob);
        handle
    }

    pub fn get<'a, T, S>(&'a self, handle: Value<Handle<H, S>>) -> Option<T>
    where
        S: BlobSchema + 'a,
        T: FromBlob<'a, S>,
    {
        let hash: Value<Hash<_>> = handle.into();
        let handle: Value<Handle<H, UnknownBlob>> = hash.into();
        let Some(blob) = self.blobs.get(&handle) else {
            return None;
        };

        Some(FromBlob::from_blob(blob.as_transmute()))
    }

    pub fn get_blob<'a, S>(&'a self, handle: Value<Handle<H, S>>) -> Option<&'a Blob<S>>
    where
        S: BlobSchema + 'a,
    {
        let hash: Value<Hash<_>> = handle.into();
        let handle: Value<Handle<H, UnknownBlob>> = hash.into();
        self.blobs.get(&handle).map(Blob::as_transmute)
    }

    pub fn insert_blob<S>(&mut self, blob: Blob<S>)
    where
        S: BlobSchema,
    {
        let handle: Value<Handle<H, S>> = blob.get_handle();
        let unknown_handle: Value<Handle<H, UnknownBlob>> = handle.transmute();
        let blob: Blob<UnknownBlob> = blob.transmute();
        self.blobs.insert(unknown_handle, blob);
    }

    pub fn iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a Value<Handle<H, UnknownBlob>>, &'a Blob<UnknownBlob>)> + 'a {
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
        self.blobs.retain(|k, _| tribles.vae.has_prefix(&k.raw));
    }
}

impl<H> FromIterator<(Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>)> for BlobSet<H>
where
    H: HashProtocol,
{
    fn from_iter<I: IntoIterator<Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>)>>(
        iter: I,
    ) -> Self {
        let mut set = BlobSet::new();

        for (handle, blob) in iter {
            set.blobs.insert(handle, blob);
        }

        set
    }
}

impl<'a, H> IntoIterator for BlobSet<H>
where
    H: HashProtocol,
{
    type Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>);
    type IntoIter =
        std::collections::hash_map::IntoIter<Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>>;

    fn into_iter(self) -> Self::IntoIter {
        self.blobs.into_iter()
    }
}

impl<'a, H> IntoIterator for &'a BlobSet<H>
where
    H: HashProtocol,
{
    type Item = (&'a Value<Handle<H, UnknownBlob>>, &'a Blob<UnknownBlob>);
    type IntoIter =
        std::collections::hash_map::Iter<'a, Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.blobs).into_iter()
    }
}

#[derive(Debug)]
pub struct NotFoundErr();

impl fmt::Display for NotFoundErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no blob for hash in blobset")
    }
}

impl Error for NotFoundErr {}

pub struct BlobSetListIter<'a, H>
where H: HashProtocol {
    iter: std::collections::hash_map::Iter<'a, Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>>,
}

impl<'a, H> Iterator for BlobSetListIter<'a, H>
where
    H: HashProtocol,
{
    type Item = Result<Value<Handle<H, UnknownBlob>>, Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, _)| Ok(k.clone()))
    }
}

impl<H> BlobStoreListOp<H> for BlobSet<H>
where
    H: HashProtocol,
{
    type Iter<'a> = BlobSetListIter<'a, H>;
    type Err = Infallible;

    fn list<'a>(&'a self) -> Self::Iter<'a> {
        BlobSetListIter {
            iter: self.blobs.iter()
        }
    }
}

impl<H> BlobStoreGetOp<H> for BlobSet<H>
where
    H: HashProtocol,
{
    type Err = NotFoundErr;

    fn get<T>(&self, handle: Value<Handle<H, T>>) -> Result<Blob<T>, Self::Err>
    where
        T: BlobSchema,
    {
        self.get(handle).ok_or(NotFoundErr())
    }
}

impl<H> BlobStorePutOp<H> for BlobSet<H>
where
    H: HashProtocol,
{
    type Err = Infallible;

    fn put<T>(&mut self, blob: Blob<T>) -> Result<Value<Handle<H, T>>, Self::Err>
    where
        T: BlobSchema,
    {
        let handle = blob.get_handle();
        self.insert_blob(blob);
        Ok(handle)
    }
}

impl<H: HashProtocol> BlobStorage<H> for BlobSet<H> {}

#[cfg(test)]
mod tests {
    use crate::{
        blob::schemas::longstring::LongString,
        trible::TribleSet,
        value::schemas::hash::{Blake3, Handle},
        NS,
    };

    use super::*;
    use anybytes::Bytes;
    use fake::{faker::name::raw::Name, locales::EN, Fake};

    NS! {
        pub namespace knights2 {
            "5AD0FAFB1FECBC197A385EC20166899E" as description: Handle<Blake3, LongString>;
        }
    }

    #[test]
    fn union() {
        let mut blobs_a: BlobSet<Blake3> = BlobSet::new();
        let mut blobs_b: BlobSet<Blake3> = BlobSet::new();

        for _i in 0..1000 {
            blobs_a.insert(
                Bytes::from_source(Name(EN).fake::<String>())
                    .view()
                    .unwrap(),
            );
        }
        for _i in 0..1000 {
            blobs_b.insert(
                Bytes::from_source(Name(EN).fake::<String>())
                    .view()
                    .unwrap(),
            );
        }

        blobs_a.union(blobs_b);
    }

    #[test]
    fn keep() {
        let mut kb = TribleSet::new();
        let mut blobs = BlobSet::new();
        for _i in 0..2000 {
            kb.union(knights2::entity!({
                description: blobs.insert(Bytes::from_source(Name(EN).fake::<String>()).view().unwrap())
            }));
        }
        blobs.keep(kb);
    }
}
