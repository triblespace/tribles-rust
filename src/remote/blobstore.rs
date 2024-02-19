use std::collections::HashMap;

use crate::{
    types::{syntactic::Hash, Blob},
    BlobSet,
};

pub struct PutError<H, E> {
    pub remaining: BlobSet<H>,
    pub causes: HashMap<Hash<H>, E>
}

pub trait BlobStore<H> {
    type Err;

    async fn put(&self, blobs: BlobSet<H>) -> Result<(), PutError<H, Self::Err>>;
    async fn get(&self, hash: Hash<H>) -> Blob;
}
