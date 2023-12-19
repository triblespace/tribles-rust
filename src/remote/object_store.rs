use std::marker::PhantomData;

use object_store::{self, parse_url, path::Path, ObjectStore, PutResult};
use url::Url;

use crate::BlobSet;

pub struct ObjectStoreRemote<H> {
    store: Box<dyn ObjectStore>,
    prefix: Path,
    _hasher: PhantomData<H>,
}

impl<H> ObjectStoreRemote<H> {
    pub fn with_url(url: &Url) -> Result<ObjectStoreRemote<H>, object_store::Error> {
        let (store, path) = parse_url(&url)?;
        Ok(ObjectStoreRemote {
            store,
            prefix: path,
            _hasher: PhantomData,
        })
    }

    /*
    fn push(&self, blobs: BlobSet<H>) -> Result<PutResult, object_store::Error> {
        let path = self.prefix.join(hex::encode());
        object_store.put(&path, bytes).await.unwrap();
    }

    pull(handle: Handle<>) {

    }
    */
}
