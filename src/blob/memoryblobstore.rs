use crate::blob::ToBlob;
use crate::blob::{schemas::UnknownBlob, Blob, BlobSchema};
use crate::repo::{BlobStore, BlobStoreGet, BlobStoreList, BlobStorePut};
use crate::trible::TribleSet;
use crate::value::schemas::hash::{Handle, HashProtocol};
use crate::value::Value;

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Debug};
use std::iter::FromIterator;
use std::ops::Bound;

use reft_light::{Apply, ReadHandle, WriteHandle};

use super::TryFromBlob;

enum MemoryBlobStoreOps<H: HashProtocol> {
    Insert(Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>),
    Keep(TribleSet),
}

type MemoryBlobStoreMap<H> = BTreeMap<Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>>;

impl<H: HashProtocol> Apply<MemoryBlobStoreMap<H>, ()> for MemoryBlobStoreOps<H> {
    fn apply_first(
        &mut self,
        first: &mut MemoryBlobStoreMap<H>,
        _second: &MemoryBlobStoreMap<H>,
        _auxiliary: &mut (),
    ) {
        match self {
            MemoryBlobStoreOps::Insert(handle, blob) => {
                // This operation is indempotent, so we can just
                // ignore it if the blob is already present.
                first.entry(*handle).or_insert(blob.clone());
            }
            MemoryBlobStoreOps::Keep(trible_set) => {
                first.retain(|k, _| trible_set.vae.has_prefix(&k.raw))
            }
        }
    }
}

/// A mapping from [Handle]s to [Blob]s.
pub struct MemoryBlobStore<H: HashProtocol> {
    write_handle: WriteHandle<MemoryBlobStoreOps<H>, MemoryBlobStoreMap<H>, ()>,
}

impl<H: HashProtocol> Debug for MemoryBlobStore<H>
where
    H: HashProtocol,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MemoryBlobStore")
    }
}

#[derive(Debug)]
pub struct MemoryBlobStoreReader<H: HashProtocol> {
    read_handle: ReadHandle<MemoryBlobStoreMap<H>>,
}

impl<H: HashProtocol> Clone for MemoryBlobStoreReader<H> {
    fn clone(&self) -> Self {
        MemoryBlobStoreReader {
            read_handle: self.read_handle.clone(),
        }
    }
}

impl<H: HashProtocol> PartialEq for MemoryBlobStoreReader<H> {
    fn eq(&self, other: &Self) -> bool {
        self.read_handle == other.read_handle
    }
}

impl<H: HashProtocol> Eq for MemoryBlobStoreReader<H> {}

impl<H: HashProtocol> MemoryBlobStoreReader<H> {
    fn new(read_handle: ReadHandle<MemoryBlobStoreMap<H>>) -> Self {
        MemoryBlobStoreReader { read_handle }
    }

    pub fn len(&self) -> usize {
        self.read_handle
            .enter()
            .map(|blobs| blobs.len())
            .unwrap_or(0)
    }

    pub fn iter(&self) -> MemoryBlobStoreIter<H> {
        let read_handle = self.read_handle.clone();
        let iter = MemoryBlobStoreIter {
            read_handle,
            cursor: None,
        };
        iter
    }
}

impl<H: HashProtocol> MemoryBlobStore<H> {
    pub fn new() -> MemoryBlobStore<H> {
        let write_storage = reft_light::new::<MemoryBlobStoreOps<H>, MemoryBlobStoreMap<H>, ()>(
            MemoryBlobStoreMap::new(),
            (),
        );
        MemoryBlobStore {
            write_handle: write_storage,
        }
    }

    pub fn insert<S>(&mut self, blob: Blob<S>) -> Value<Handle<H, S>>
    where
        S: BlobSchema,
    {
        let handle: Value<Handle<H, S>> = blob.get_handle();
        let unknown_handle: Value<Handle<H, UnknownBlob>> = handle.transmute();
        let blob: Blob<UnknownBlob> = blob.transmute();
        self.write_handle
            .append(MemoryBlobStoreOps::Insert(unknown_handle, blob));
        handle
    }

    // Note that keep is conservative and keeps every blob for which there exists
    // a corresponding trible value, irrespective of that tribles attribute type.
    // This could theoretically allow an attacker to DOS blob garbage collection
    // by introducting values that look like existing hashes, but are actually of
    // a different type. But this is under the assumption that an attacker is only
    // allowed to write non-handle typed triples, otherwise they might as well
    // introduce blobs directly.
    pub fn keep(&mut self, tribles: TribleSet) {
        self.write_handle.append(MemoryBlobStoreOps::Keep(tribles));
    }
}

impl<H> FromIterator<(Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>)> for MemoryBlobStore<H>
where
    H: HashProtocol,
{
    fn from_iter<I: IntoIterator<Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>)>>(
        iter: I,
    ) -> Self {
        let mut set = MemoryBlobStore::new();

        for (handle, blob) in iter {
            set.write_handle
                .append(MemoryBlobStoreOps::Insert(handle, blob));
        }

        set
    }
}

impl<'a, H> IntoIterator for MemoryBlobStoreReader<H>
where
    H: HashProtocol,
{
    type Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>);
    type IntoIter = MemoryBlobStoreIter<H>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug)]
pub enum MemoryStoreGetError<E: Error> {
    /// This error occurs when a blob is requested that does not exist in the store.
    /// It is used to indicate that the requested blob could not be found.
    NotFound(),
    /// This error occurs when a blob is requested that exists, but cannot be converted to the requested type.
    ConversionFailed(E),
}

impl<E: Error> fmt::Display for MemoryStoreGetError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryStoreGetError::NotFound() => write!(f, "Blob not found in memory store"),
            MemoryStoreGetError::ConversionFailed(e) => write!(f, "Blob conversion failed: {}", e),
        }
    }
}

impl<E: Error> Error for MemoryStoreGetError<E> {}

pub struct MemoryBlobStoreIter<H>
where
    H: HashProtocol,
{
    read_handle: ReadHandle<MemoryBlobStoreMap<H>>,
    cursor: Option<Value<Handle<H, UnknownBlob>>>,
}

impl<'a, H> Iterator for MemoryBlobStoreIter<H>
where
    H: HashProtocol,
{
    type Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>);

    fn next(&mut self) -> Option<Self::Item> {
        let read_handle = self.read_handle.enter()?;
        let mut iter = if let Some(cursor) = self.cursor.take() {
            // If we have a cursor, we start from the cursor.
            // We use `Bound::Excluded` to skip the cursor itself.
            read_handle.range((Bound::Excluded(cursor), Bound::Unbounded))
        } else {
            // If we don't have a cursor, we start from the beginning.
            read_handle.range((
                Bound::Unbounded::<Value<Handle<H, UnknownBlob>>>,
                Bound::Unbounded,
            ))
        };

        let (handle, blob) = iter.next()?;
        self.cursor = Some(handle.clone());
        return Some((handle.clone(), blob.clone()));
        //TODO we may want to use batching in the future to gain more performance and amortize
        // the cost of creating the iterator over the BTreeMap.
    }
}

pub struct MemoryBlobStoreListIter<H>
where
    H: HashProtocol,
{
    inner: MemoryBlobStoreIter<H>,
}

impl<H> Iterator for MemoryBlobStoreListIter<H>
where
    H: HashProtocol,
{
    type Item = Result<Value<Handle<H, UnknownBlob>>, Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        let (handle, _) = self.inner.next()?;
        Some(Ok(handle))
    }
}

impl<H> BlobStoreList<H> for MemoryBlobStoreReader<H>
where
    H: HashProtocol,
{
    type Iter<'a> = MemoryBlobStoreListIter<H>;
    type Err = Infallible;

    fn blobs(&self) -> Self::Iter<'static> {
        MemoryBlobStoreListIter { inner: self.iter() }
    }
}

impl<H> BlobStoreGet<H> for MemoryBlobStoreReader<H>
where
    H: HashProtocol,
{
    type GetError<E: Error> = MemoryStoreGetError<E>;

    fn get<T, S>(
        &self,
        handle: Value<Handle<H, S>>,
    ) -> Result<T, Self::GetError<<T as TryFromBlob<S>>::Error>>
    where
        S: BlobSchema,
        T: TryFromBlob<S>,
    {
        let handle: Value<Handle<H, UnknownBlob>> = handle.transmute();

        let Some(read_guard) = self.read_handle.enter() else {
            return Err(MemoryStoreGetError::NotFound());
        };
        let Some(blob) = read_guard.get(&handle) else {
            return Err(MemoryStoreGetError::NotFound());
        };

        let blob: Blob<S> = blob.clone().transmute();

        match blob.try_from_blob() {
            Ok(value) => Ok(value),
            Err(e) => Err(MemoryStoreGetError::ConversionFailed(e)),
        }
    }
}

impl<H> BlobStorePut<H> for MemoryBlobStore<H>
where
    H: HashProtocol,
{
    type PutError = Infallible;

    fn put<S, T>(&mut self, item: T) -> Result<Value<Handle<H, S>>, Self::PutError>
    where
        S: BlobSchema,
        T: ToBlob<S>,
    {
        let blob = item.to_blob();
        let handle = blob.get_handle();
        self.insert(blob);
        Ok(handle)
    }
}

impl<H: HashProtocol> BlobStore<H> for MemoryBlobStore<H> {
    type Reader = MemoryBlobStoreReader<H>;

    fn reader(&mut self) -> Self::Reader {
        MemoryBlobStoreReader::new(self.write_handle.publish().clone())
    }
}

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
    fn keep() {
        let mut kb = TribleSet::new();
        let mut blobs = MemoryBlobStore::new();
        for _i in 0..2000 {
            kb.union(knights2::entity!({
                description: blobs.put(Bytes::from_source(Name(EN).fake::<String>()).view().unwrap()).unwrap()
            }));
        }
        blobs.keep(kb);
    }
}
