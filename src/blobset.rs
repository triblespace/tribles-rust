//use crate::trible::{Id, Value, VALUE_LEN};
use crate::patch::{Entry, PATCH};
use crate::trible::{Trible, TribleSegmentation, VAEOrder, VALUE_LEN, Blob,};
use crate::types::handle::Handle;
use std::iter::FromIterator;

#[derive(Debug, Clone)]
pub struct PATCHBlobSet {
    vae: PATCH<64, VAEOrder, TribleSegmentation, Blob>,
}

impl PATCHBlobSet {
    pub fn union<'a>(&mut self, other: &Self) {
        self.vae.union(&other.vae);
    }

    pub fn new() -> PATCHBlobSet {
        PATCHBlobSet {
            vae: PATCH::new(),
        }
    }

    pub fn len(&self) -> u64 {
        return self.vae.segmented_len(&[0; 64], 0);
    }

    pub fn insert(&mut self, trible: &Trible, blob: Blob) {
        let key = Entry::new(&trible.data, blob);
        self.vae.insert(&key);
    }

    pub fn get<T>(&mut self, handle: Handle<T>) -> Option<T>
    where T: std::convert::From<Blob> {
        let t = Trible::new_raw_values([0; 32], [0; 32], handle.value);
        let blob = self.vae.any_prefixed_value(&t.data, VALUE_LEN)?;
        Some(blob.into())
    }
}

impl FromIterator<(Trible, Blob)> for PATCHBlobSet {
    fn from_iter<I: IntoIterator<Item = (Trible, Blob)>>(iter: I) -> Self {
        let mut set = PATCHBlobSet::new();

        for (t, blob) in iter {
            set.insert(&t, blob);
        }

        set
    }
}
