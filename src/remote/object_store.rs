use std::marker::PhantomData;

use futures::{future, stream::FuturesUnordered, StreamExt};

use digest::{typenum::U32, Digest, OutputSizeUser};
use object_store::{self, parse_url, path::Path, ObjectStore};
use url::Url;

use crate::{
    types::{syntactic::Hash, Blob},
    BlobSet,
};

use super::{blobstore::PutError, BlobStore};

pub struct ObjectBlobStore<H> {
    store: Box<dyn ObjectStore>,
    prefix: Path,
    _hasher: PhantomData<H>,
}

impl<H> ObjectBlobStore<H> {
    pub fn with_url(url: &Url) -> Result<ObjectBlobStore<H>, object_store::Error> {
        let (store, path) = parse_url(&url)?;
        Ok(ObjectBlobStore {
            store,
            prefix: path,
            _hasher: PhantomData,
        })
    }
}

impl<H> BlobStore<H> for ObjectBlobStore<H>
where H: Digest + OutputSizeUser<OutputSize = U32> {
    type Err = object_store::Error;

    async fn put(&self, blobs: BlobSet<H>) -> Result<(), PutError<H, Self::Err>> {
        let futures = FuturesUnordered::new();

        blobs.raw_each(|hash: Hash<H>, blob: Blob| {
            futures.push(async move {
                let path = self.prefix.child(hex::encode(hash.value));
                if let Err(err) = self.store.put(&path, blob.clone()).await {
                    Some((hash, blob, err))
                } else {
                    None
                }
            });
        });

        let mut causes = std::collections::HashMap::new();
        let mut remaining = BlobSet::new();

        futures.for_each(|r| {
            if let Some((hash, blob, err)) = r {
                causes.insert(hash, err);
                remaining.raw_put(hash, blob);
            }
            future::ready(())
        }).await;

        if causes.is_empty() {
            Ok(())
        } else {
            Err(PutError {
                remaining,
                causes
            })
        }
    }

    async fn get(&self, hash: Hash<H>) -> Blob {
        let path = self.prefix.child(hex::encode(hash.value));
        let result = self.store.get(&path).await.unwrap();
        let object = result.bytes().await.unwrap();
        object
    }
}

pub struct ObjectHead<H> {
    store: Box<dyn ObjectStore>,
    prefix: Path,
    _hasher: PhantomData<H>,
}

/*
impl<H> ObjectHead<H> {
    pub fn with_url(url: &Url) -> Result<ObjectBlobStore<H>, object_store::Error> {
        let (store, path) = parse_url(&url)?;
        Ok(ObjectBlobStore {
            store,
            prefix: path,
            _hasher: PhantomData,
        })
    }

    async fn get(&self, hash: Hash<H>) -> Blob {
        let path = self.prefix.child("blobs").child(hex::encode(hash.value));
        let result = self.store.get(&path).await.unwrap();
        let object = result.bytes().await.unwrap();
        object
    }

    async fn push<F>(&self, do_update: F) -> Result<PutResult, object_store::Error>
    where
        F: Fn(Handle<H, TribleSet>) -> TribleSet,
    {
        let path = self.prefix.child("heads").child(head);

        loop {
            // Perform get request
            let r = self.store.get(&path).await.unwrap();

            // Save version information fetched
            let version = UpdateVersion {
                e_tag: r.meta.e_tag.clone(),
                version: r.meta.version.clone(),
            };

            // Compute new version of object contents
            let new = do_update(r.bytes().await.unwrap());

            // Attempt to commit transaction
            match self
                .store
                .put_opts(&path, new, PutMode::Update(version).into())
                .await
            {
                Ok(r) => return Ok(r),                       // Successfully committed
                Err(Error::Precondition { .. }) => continue, // Object has changed, try again
                Err(e) => return Err(e),
            }
        }
    }
}
*/