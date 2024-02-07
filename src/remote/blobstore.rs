use anyhow::Error;

use crate::{
    types::{syntactic::Hash, Blob},
    BlobSet,
};

pub trait BlobStore<H> {
    async fn put(&self, blobs: BlobSet<H>) -> Result<(), Error>;
    async fn get(&self, hash: Hash<H>) -> Blob;
}
