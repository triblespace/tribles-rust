use digest::typenum::U32;
use digest::{Digest, OutputSizeUser};

use crate::namespace::triblepattern::TriblePattern;
use crate::patch::{Entry, IdentityOrder, SingleSegmentation, PATCH};
use crate::types::handle::Handle;
use crate::types::syntactic::{Hash, UFOID};
use crate::types::{Blob, Bloblike, Value, VALUE_LEN};
use crate::{and, mask, query};
use std::iter::FromIterator;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct BlobSet<H> {
    blobs: PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, Blob>,
    _hasher: PhantomData<H>,
}

impl<H> Eq for BlobSet<H> {}

impl<H> PartialEq for BlobSet<H> {
    fn eq(&self, other: &Self) -> bool {
        self.blobs == other.blobs
    }
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
            _hasher: PhantomData,
        }
    }

    pub fn len(&self) -> u64 {
        return self.blobs.segmented_len(&[0; VALUE_LEN], 0);
    }

    pub fn put<V>(&mut self, value: V) -> Handle<H, V>
    where
        V: Bloblike,
    {
        let blob: Blob = value.into_blob();
        let hash = H::digest(&blob).into();
        let entry = Entry::new(&hash, blob);
        self.blobs.insert(&entry);
        Handle::new(hash)
    }

    pub fn put_raw(&mut self, hash: Hash<H>, blob: Blob) {
        let entry = Entry::new(&hash.value, blob);
        self.blobs.insert(&entry);
    }

    pub fn get<T>(&self, handle: Handle<H, T>) -> Option<T>
    where
        T: std::convert::From<Blob>,
    {
        let blob = self.blobs.get(&handle.hash.value)?;
        Some(blob.into())
    }

    pub fn get_raw(&self, value: &Value) -> Option<Blob> {
        self.blobs.get(value)
    }

    // Note that keep is conservative and keeps every blob for which there exists
    // a corresponding trible value, irrespective of that tribles attribute type.
    // This could theoretically allow an attacker to DOS blob garbage collection
    // by introducting values that look like existing hashes, but are actually of
    // a different type. But this is under the assumption that an attacker is only
    // allowed to write non-handle typed triples, otherwise they might as well
    // introduce blobs directly.
    pub fn keep<T>(&self, tribles: T) -> BlobSet<H>
    where
        T: TriblePattern,
    {
        let mut set = BlobSet::new();

        for (hash,) in query!(
            ctx,
            (v),
            and!(
                v.of(&self.blobs),
                mask!(
                    ctx,
                    (e, a),
                    tribles.pattern::<UFOID, UFOID, Hash<H>>(e, a, v)
                )
            )
        ).flatten() {
            let blob = self.blobs.get(&hash.value).unwrap();
            set.put_raw(hash, blob)
        }

        set
    }
}

impl<H> FromIterator<(Hash<H>, Blob)> for BlobSet<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    fn from_iter<I: IntoIterator<Item = (Hash<H>, Blob)>>(iter: I) -> Self {
        let mut set = BlobSet::new();

        for (hash, blob) in iter {
            set.put_raw(hash, blob);
        }

        set
    }
}

#[cfg(test)]
mod tests {
    use crate::{TribleSet, NS};

    use super::*;
    use fake::{faker::name::raw::Name, locales::EN, Fake};

    NS! {
        pub namespace knights {
            @ crate::types::syntactic::UFOID;
            description: "5AD0FAFB1FECBC197A385EC20166899E" as crate::types::handle::Handle<
                crate::types::syntactic::Blake2b,
                crate::types::syntactic::LongString>;
        }
    }

    #[test]
    fn keep() {
        let mut kb = TribleSet::new();
        let mut blobs = BlobSet::new();
        for _i in 0..2000 {
            kb.union(&knights::entities!((entity),
            [{entity @
                description: blobs.put(Name(EN).fake::<String>().into())
            }]));
        }
        let kept = blobs.keep(kb);
        assert_eq!(blobs, kept);
    }
}
