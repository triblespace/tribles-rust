use std::array::TryFromSliceError;
use std::convert::TryInto;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;

use bytes::Bytes;
use futures::{Stream, StreamExt};

use digest::{typenum::U32, Digest, OutputSizeUser};
use object_store::UpdateVersion;
use object_store::{self, parse_url, path::Path, ObjectStore, PutMode};
use url::Url;

use hex::FromHex;

use crate::{types::Hash, Value};

use super::head::{CommitResult, Head};
use super::repo::{List, Pull, Push};

pub struct ObjectRepo<H> {
    store: Box<dyn ObjectStore>,
    prefix: Path,
    _hasher: PhantomData<H>,
}

impl<H> ObjectRepo<H> {
    pub fn with_url(url: &Url) -> Result<ObjectRepo<H>, object_store::Error> {
        let (store, path) = parse_url(&url)?;
        Ok(ObjectRepo {
            store,
            prefix: path,
            _hasher: PhantomData,
        })
    }
}

pub enum ListErr {
    List(object_store::Error),
    NotAFile(&'static str),
    BadNameHex(<Value as FromHex>::Error),
}

impl<H> List<H> for ObjectRepo<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    type Err = ListErr;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Hash<H>, Self::Err>> {
        self.store
            .list(Some(&self.prefix))
            .map(|r| match r {
                Ok(meta) => {
                    let blob_name = meta
                        .location
                        .filename()
                        .ok_or(ListErr::NotAFile("no filename"))?;
                    let digest = Value::from_hex(blob_name).map_err(|e| ListErr::BadNameHex(e))?;
                    Ok(Hash::new(digest))
                }
                Err(e) => Err(ListErr::List(e)),
            })
            .boxed()
    }
}

impl<H> Pull<H> for ObjectRepo<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    type Err = object_store::Error;

    async fn pull(&self, hash: Hash<H>) -> Result<Bytes, Self::Err> {
        let path = self.prefix.child(hex::encode(hash.bytes));
        let result = self.store.get(&path).await?;
        let object = result.bytes().await?;
        Ok(object)
    }
}

impl<H> Push<H> for ObjectRepo<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    type Err = object_store::Error;

    async fn push(&self, blob: Bytes) -> Result<Hash<H>, Self::Err> {
        let digest: Value = H::digest(&blob).into();
        let path = self.prefix.child(hex::encode(digest));
        let put_result = self
            .store
            .put_opts(&path, blob.clone(), PutMode::Create.into())
            .await;
        match put_result {
            Ok(_) | Err(object_store::Error::AlreadyExists { .. }) => Ok(Hash::new(digest)),
            Err(e) => Err(e),
        }
    }
}

pub struct ObjectHead<H> {
    store: Box<dyn ObjectStore>,
    path: Path,
    _hasher: PhantomData<H>,
}

impl<H> ObjectHead<H> {
    pub fn with_url(url: &Url) -> Result<ObjectHead<H>, object_store::Error> {
        let (store, path) = parse_url(&url)?;
        Ok(ObjectHead {
            store,
            path,
            _hasher: PhantomData,
        })
    }
}

#[derive(Debug)]
pub enum CheckoutErr {
    ValidationErr(TryFromSliceError),
    StoreErr(object_store::Error),
}

impl fmt::Display for CheckoutErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::StoreErr(e) => write!(f, "checkout failed: {}", e),
            Self::ValidationErr(e) => write!(f, "checkout failed: {}", e),
        }
    }
}

impl Error for CheckoutErr {}

impl From<object_store::Error> for CheckoutErr {
    fn from(err: object_store::Error) -> Self {
        Self::StoreErr(err)
    }
}

impl From<TryFromSliceError> for CheckoutErr {
    fn from(err: TryFromSliceError) -> Self {
        Self::ValidationErr(err)
    }
}

#[derive(Debug)]
pub enum CommitErr {
    ValidationErr(TryFromSliceError),
    StoreErr(object_store::Error),
}

impl fmt::Display for CommitErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ValidationErr(e) => write!(f, "commit failed: {}", e),
            Self::StoreErr(e) => write!(f, "commit failed: {}", e),
        }
    }
}

impl Error for CommitErr {}

impl From<object_store::Error> for CommitErr {
    fn from(err: object_store::Error) -> Self {
        Self::StoreErr(err)
    }
}

impl From<TryFromSliceError> for CommitErr {
    fn from(err: TryFromSliceError) -> Self {
        Self::ValidationErr(err)
    }
}

impl<H> Head<H> for ObjectHead<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    type CheckoutErr = CheckoutErr;
    type CommitErr = CommitErr;

    async fn checkout(&self) -> Result<Option<Hash<H>>, Self::CheckoutErr> {
        let result = self.store.get(&self.path).await;
        match result {
            Ok(result) => {
                let bytes = result.bytes().await?;
                let value = (&bytes[..]).try_into()?;
                Ok(Some(Hash::new(value)))
            }
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(e)?,
        }
    }

    async fn commit(
        &self,
        old_hash: Option<Hash<H>>,
        new_hash: Hash<H>,
    ) -> Result<CommitResult<H>, Self::CommitErr> {
        let new_bytes = Bytes::copy_from_slice(&new_hash.bytes);

        if let Some(old_hash) = old_hash {
            let mut result = self.store.get(&self.path).await;
            loop {
                // Attempt to commit
                match result {
                    Ok(ok_result) => {
                        // Save version information fetched
                        let version = UpdateVersion {
                            e_tag: ok_result.meta.e_tag.clone(),
                            version: ok_result.meta.version.clone(),
                        };
                        let stored_bytes = ok_result.bytes().await?;
                        let stored_value = (&stored_bytes[..]).try_into()?;
                        let stored_hash = Hash::new(stored_value);
                        if old_hash != stored_hash {
                            return Ok(CommitResult::Conflict(Some(stored_hash)));
                        }
                        match self
                            .store
                            .put_opts(
                                &self.path,
                                new_bytes.clone(),
                                PutMode::Update(version).into(),
                            )
                            .await
                        {
                            Ok(_) => return Ok(CommitResult::Success()), // Successfully committed
                            Err(object_store::Error::Precondition { .. }) => {
                                result = self.store.get(&self.path).await;
                                continue;
                            }
                            Err(e) => return Err(e.into()),
                        }
                    }
                    Err(object_store::Error::NotFound { .. }) => {
                        return Ok(CommitResult::Conflict(None));
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        } else {
            loop {
                // Attempt to commit
                match self
                    .store
                    .put_opts(&self.path, new_bytes.clone(), PutMode::Create.into())
                    .await
                {
                    Ok(_) => return Ok(CommitResult::Success()), // Successfully committed
                    Err(object_store::Error::AlreadyExists { .. }) => {
                        let result = self.store.get(&self.path).await;
                        match result {
                            Ok(result) => {
                                let stored_bytes = result.bytes().await?;
                                let stored_value = (&stored_bytes[..]).try_into()?;
                                return Ok(CommitResult::Conflict(Some(Hash::new(stored_value))));
                            }
                            Err(object_store::Error::NotFound { .. }) => {
                                continue; // Object no longer exists try again
                            }
                            Err(e) => return Err(e.into()),
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
}
