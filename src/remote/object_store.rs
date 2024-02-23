use std::marker::PhantomData;

use futures::{stream::BoxStream, StreamExt};

use digest::{typenum::U32, Digest, OutputSizeUser};
use object_store::{self, parse_url, path::Path, ObjectStore, PutMode};
use url::Url;

use hex::FromHex;

use crate::types::{syntactic::Hash, Blob, Value};

use super::BlobStore;

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

pub enum ListErr {
    List(object_store::Error),
    NotAFile(&'static str),
    BadNameHex(<Value as FromHex>::Error)
}

impl<H> BlobStore<H> for ObjectBlobStore<H>
where H: Digest + OutputSizeUser<OutputSize = U32> {
    type StoreErr = object_store::Error;
    type LoadErr = object_store::Error;
    type ListErr = ListErr;
    type ListStream<'a> = BoxStream<'a, Result<Hash<H>, Self::ListErr>>
    where Self: 'a;

    async fn put_raw(&self, blob: Blob) -> Result<Hash<H>, Self::StoreErr> {
        let digest: Value = H::digest(&blob).into();
        let path = self.prefix.child(hex::encode(digest));
        let put_result = self.store.put_opts(&path, blob.clone(), PutMode::Create.into()).await;
        match put_result {
            Ok(_) | Err(object_store::Error::AlreadyExists {..}) => Ok(Hash::new(digest)),
            Err(e) => Err(e)
        }
    }

    async fn get_raw(&self, hash: Hash<H>) -> Result<Blob, Self::LoadErr> {
        let path = self.prefix.child(hex::encode(hash.value));
        let result = self.store.get(&path).await?;
        let object = result.bytes().await?;
        Ok(object)
    }

    fn list<'a>(&'a self) -> Self::ListStream<'a> {
        self.store.list(Some(&self.prefix)).map(|r| {
            match r {
                Ok(meta) => {
                    let blob_name = meta.location.filename().ok_or(ListErr::NotAFile("no filename"))?;
                    let digest = Value::from_hex(blob_name).map_err(|e| ListErr::BadNameHex(e))?;
                    Ok(Hash::new(digest))
                }
                Err(e) => Err(ListErr::List(e))
            }
        }).boxed()
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