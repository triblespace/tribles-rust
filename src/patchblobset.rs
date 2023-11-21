//use crate::trible::{Id, Value, VALUE_LEN};
use crate::patch::{Entry, PATCH, IdentityOrder, SingleSegmentation};
use crate::query::Variable;
use crate::types::syntactic::{UFOID, RawValue};
use crate::{query, and, mask};
use crate::trible::{VALUE_LEN, Blob, Value, Id,};
use crate::tribleset::TribleSet;
use crate::types::handle::Handle;
use std::iter::FromIterator;

#[derive(Debug, Clone)]
pub struct PATCHBlobSet {
    blobs: PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, Blob>,
}

impl PATCHBlobSet {
    pub fn union<'a>(&mut self, other: &Self) {
        self.blobs.union(&other.blobs);
    }

    pub fn new() -> PATCHBlobSet {
        PATCHBlobSet {
            blobs: PATCH::new(),
        }
    }

    pub fn len(&self) -> u64 {
        return self.blobs.segmented_len(&[0; VALUE_LEN], 0);
    }

    pub fn insert<V>(&mut self, value: V) -> Handle<V>
    where V: Into<Blob>,
          for<'a> Handle<V>: From<&'a Blob> {
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
    where T: std::convert::From<Blob> {
        let blob = self.blobs.get(&handle.value)?;
        Some(blob.into())
    }

    pub fn get_raw(&self, value: &Value) -> Option<Blob> {
        self.blobs.get(value)
    }

    pub fn keep<S>(&self, tribles: S) -> PATCHBlobSet
    where S: TribleSet {
        let mut set = PATCHBlobSet::new();
        for r in query!(ctx, (v),
            and!(v.of(&self.blobs),
                 mask!(ctx, (e, a), tribles.pattern::<UFOID, UFOID, RawValue>(e, a, v)))) {
                    let (RawValue(value),) = r;
                    let blob = self.blobs.get(&value).unwrap();
                    set.insert_raw(value, blob)
                 }
        set
    }
}

impl FromIterator<(Value, Blob)> for PATCHBlobSet {
    fn from_iter<I: IntoIterator<Item = (Value, Blob)>>(iter: I) -> Self {
        let mut set = PATCHBlobSet::new();

        for (value, blob) in iter {
            set.insert_raw(value, blob);
        }

        set
    }
}
