use crate::namespace::triblepattern::TriblePattern;
use crate::patch::{Entry, IdentityOrder, SingleSegmentation, PATCH};
use crate::trible::{Blob, Value, VALUE_LEN};
use crate::types::handle::Handle;
use crate::types::syntactic::{RawValue, UFOID};
use crate::{and, mask, query};
use std::iter::FromIterator;

#[derive(Debug, Clone)]
pub struct BlobSet {
    blobs: PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, Blob>,
}

impl BlobSet {
    pub fn union<'a>(&mut self, other: &Self) {
        self.blobs.union(&other.blobs);
    }

    pub fn new() -> BlobSet {
        BlobSet {
            blobs: PATCH::new(),
        }
    }

    pub fn len(&self) -> u64 {
        return self.blobs.segmented_len(&[0; VALUE_LEN], 0);
    }

    pub fn insert<V>(&mut self, value: V) -> Handle<V>
    where
        V: Into<Blob>,
        for<'a> Handle<V>: From<&'a Blob>,
    {
        let blob: Blob = value.into();
        let handle: Handle<V> = (&blob).into();
        let entry = Entry::new(&handle.value, blob);
        self.blobs.insert(&entry);
        handle
    }

    pub fn insert_raw(&mut self, value: Value, blob: Blob) {
        let entry = Entry::new(&value, blob);
        self.blobs.insert(&entry);
    }

    pub fn get<T>(&self, handle: Handle<T>) -> Option<T>
    where
        T: std::convert::From<Blob>,
    {
        let blob = self.blobs.get(&handle.value)?;
        Some(blob.into())
    }

    pub fn get_raw(&self, value: &Value) -> Option<Blob> {
        self.blobs.get(value)
    }

    pub fn keep<T>(&self, tribles: T) -> BlobSet
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

impl FromIterator<(Value, Blob)> for BlobSet {
    fn from_iter<I: IntoIterator<Item = (Value, Blob)>>(iter: I) -> Self {
        let mut set = BlobSet::new();

        for (value, blob) in iter {
            set.insert_raw(value, blob);
        }

        set
    }
}
