use std::{collections::HashMap, marker::PhantomData};

use anyhow::Error;

use object_store::{self, parse_url, path::Path, ObjectStore, PutResult, UpdateVersion};
use url::Url;

use crate::{
    types::{handle::Handle, syntactic::Hash, Blob},
    BlobSet, TribleSet,
};

use super::BlobStore;

/*
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

pub struct PutError<H> {
    remaining: BlobSet<H>,
    errors: HashMap<H, Error>
}

impl<H> BlobStore<H> for ObjectBlobStore<H> {
    async fn put(&self, blobs: BlobSet<H>) -> Result<(), Error> {
        blobs.iter().map() {
            let path = self.prefix.child("blobs").child(hex::encode());
            self.store.put(&path, bytes).await?;
        }
        Ok(())
    }

    async fn get(&self, hash: Hash<H>) -> Blob {
        let path = self.prefix.child("blobs").child(hex::encode(hash.value));
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
