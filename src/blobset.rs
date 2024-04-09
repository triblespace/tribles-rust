use bytes::Bytes;
use digest::typenum::U32;
use digest::{Digest, OutputSizeUser};

use crate::patch::{Entry, IdentityOrder, PATCHIterator, SingleSegmentation, PATCH};
use crate::query::TriblePattern;
use crate::types::Hash;
use crate::Handle;
use crate::{and, mask, query::find};
use crate::{BlobParseError, Bloblike, Value, VALUE_LEN};
use std::iter::FromIterator;
use std::marker::PhantomData;

/// A pesistent mapping from [Handle]s to [Blob]s.
#[derive(Debug, Clone)]
pub struct BlobSet<H> {
    blobs: PATCH<VALUE_LEN, IdentityOrder, SingleSegmentation, Bytes>,
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
        self.blobs.len()
    }

    pub fn put<T>(&mut self, value: T) -> Handle<H, T>
    where
        T: for<'a> Bloblike<'a>,
    {
        let blob: Bytes = value.into_blob();
        let hash = self.put_raw(blob);
        unsafe { Handle::new(hash) }
    }

    pub fn get<'a, T>(&'a self, handle: Handle<H, T>) -> Option<Result<T::Read, BlobParseError>>
    where
        T: Bloblike<'a>,
    {
        let blob = self.get_raw(handle.hash)?;
        Some(T::read_blob(blob))
    }

    pub fn get_raw(&self, hash: Hash<H>) -> Option<&Bytes> {
        self.blobs.get(&hash.bytes)
    }

    pub fn put_raw(&mut self, blob: Bytes) -> Hash<H> {
        let digest = H::digest(&blob).into();
        let entry = Entry::new(&digest, blob);
        self.blobs.insert(&entry);
        Hash::new(digest)
    }

    pub fn each_raw<F>(&self, mut f: F)
    where
        F: FnMut(Hash<H>, Bytes),
    {
        self.blobs.infixes(&[0; 0], &mut |infix: [u8; VALUE_LEN]| {
            let h: Hash<H> = Hash::new(infix);
            let b = self.blobs.get(&infix).unwrap().clone();
            f(h, b);
        });
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

        for (hash,) in find!(
            ctx,
            (v),
            and!(
                v.of(&self.blobs),
                mask!(ctx, (e, a), tribles.pattern::<Hash<H>>(e, a, v))
            )
        )
        .flatten()
        {
            let blob = self.blobs.get(&hash.bytes).unwrap().clone();
            let entry = Entry::new(&hash.bytes, blob);
            set.blobs.insert(&entry);
        }

        set
    }
}

impl<H> FromIterator<(Hash<H>, Bytes)> for BlobSet<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    fn from_iter<I: IntoIterator<Item = (Hash<H>, Bytes)>>(iter: I) -> Self {
        let mut set = BlobSet::new();

        for (hash, blob) in iter {
            let entry = Entry::new(&hash.bytes, blob);
            set.blobs.insert(&entry);
        }

        set
    }
}

impl<'a, H> IntoIterator for &'a BlobSet<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    type Item = (Hash<H>, &'a Bytes);
    //TODO replace this with `impl` once https://github.com/rust-lang/rust/pull/120700 drops!
    type IntoIter = std::iter::Map<
        PATCHIterator<'a, VALUE_LEN, IdentityOrder, SingleSegmentation, Bytes>,
        fn((Value, &'a Bytes)) -> (Hash<H>, &'a Bytes),
    >;

    fn into_iter(self) -> Self::IntoIter {
        (&self.blobs)
            .into_iter()
            .map(|(value, blob)| (Hash::new(value), blob))
    }
}

#[cfg(test)]
mod tests {
    use crate::{TribleSet, NS};

    use super::*;
    use fake::{faker::name::raw::Name, locales::EN, Fake};

    NS! {
        pub namespace knights {
            "5AD0FAFB1FECBC197A385EC20166899E" as description: crate::Handle<
                crate::types::hash::Blake2b,
                String>;
        }
    }

    #[test]
    fn keep() {
        let mut kb = TribleSet::new();
        let mut blobs = BlobSet::new();
        for _i in 0..2000 {
            kb.union(&knights::entity!({
                description: blobs.put(Name(EN).fake::<String>())
            }));
        }
        let kept = blobs.keep(kb);
        assert_eq!(blobs, kept);
    }
}
