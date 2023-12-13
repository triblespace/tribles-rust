use digest::typenum::U32;
use digest::{Digest, OutputSizeUser};

use crate::namespace::triblepattern::TriblePattern;
use crate::patch::{Entry, IdentityOrder, SingleSegmentation, PATCH};
use crate::trible::{Blob, Value, VALUE_LEN};
use crate::types::handle::Handle;
use crate::types::syntactic::{RawValue, UFOID};
use crate::{and, mask, query};
use std::iter::FromIterator;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct BlobSet<H>
{
    blobs: PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, Blob>,
    _hasher: PhantomData<H>
}

impl<H> BlobSet<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    pub fn union<'a>(&mut self, other: &Self) {
        self.blobs.union(&other.blobs);
    }

    pub fn new() -> BlobSet<H> {
        BlobSet {
            blobs: PATCH::new(),
            _hasher: PhantomData
        }
    }

    pub fn len(&self) -> u64 {
        return self.blobs.segmented_len(&[0; VALUE_LEN], 0);
    }

    pub fn insert<V>(&mut self, value: V) -> Handle<H, V>
    where
        V: Into<Blob>,
        //for<'a> Handle<H, V>: From<&'a Blob>,
    {
        let blob: Blob = value.into();
        let hash = H::digest(&blob).into();
        let entry = Entry::new(&hash, blob);
        self.blobs.insert(&entry);
        Handle::new(hash)
    }

    pub fn insert_raw(&mut self, value: Value, blob: Blob) {
        let entry = Entry::new(&value, blob);
        self.blobs.insert(&entry);
    }

    pub fn get<T>(&self, handle: Handle<H, T>) -> Option<T>
    where
        T: std::convert::From<Blob>,
    {
        let blob = self.blobs.get(&handle.hash)?;
        Some(blob.into())
    }

    pub fn get_raw(&self, value: &Value) -> Option<Blob> {
        self.blobs.get(value)
    }

    pub fn keep<T>(&self, tribles: T) -> BlobSet<H>
    where
        T: TriblePattern,
    {
        let mut set = BlobSet::new();
        for (RawValue(value),) in query!(
            ctx,
            (v),
            and!(
                v.of(&self.blobs),
                mask!(
                    ctx,
                    (e, a),
                    tribles.pattern::<UFOID, UFOID, RawValue>(e, a, v)
                )
            )
        ) {
            let blob = self.blobs.get(&value).unwrap();
            set.insert_raw(value, blob)
        }
        set
    }
}

impl<H> FromIterator<(Handle<H, Blob>, Blob)> for BlobSet<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    fn from_iter<I: IntoIterator<Item = (Handle<H, Blob>, Blob)>>(iter: I) -> Self {
        let mut set = BlobSet::new();

        for (handle, blob) in iter {
            set.insert_raw(handle.hash, blob);
        }

        set
    }
}
