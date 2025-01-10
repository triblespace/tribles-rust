use std::array::TryFromSliceError;
use std::convert::TryInto;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;

use anybytes::Bytes;
use futures::{Stream, StreamExt};

use object_store::UpdateVersion;
use object_store::{self, parse_url, path::Path, ObjectStore, PutMode};
use url::Url;

use hex::FromHex;

use crate::blob::schemas::UnknownBlob;
use crate::blob::{Blob, BlobSchema};
use crate::value::{
    schemas::hash::{Handle, Hash, HashProtocol},
    RawValue, Value, ValueSchema,
};

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
    BadNameHex(<RawValue as FromHex>::Error),
}

impl<H> List<H> for ObjectRepo<H>
where
    H: HashProtocol,
{
    type Err = ListErr;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>> {
        self.store
            .list(Some(&self.prefix))
            .map(|r| match r {
                Ok(meta) => {
                    let blob_name = meta
                        .location
                        .filename()
                        .ok_or(ListErr::NotAFile("no filename"))?;
                    let digest =
                        RawValue::from_hex(blob_name).map_err(|e| ListErr::BadNameHex(e))?;
                    Ok(Value::new(digest))
                }
                Err(e) => Err(ListErr::List(e)),
            })
            .boxed()
    }
}

impl<H> Pull<H> for ObjectRepo<H>
where
    H: HashProtocol,
{
    type Err = object_store::Error;

    fn pull<T>(
        &self,
        handle: Value<Handle<H, T>>,
    ) -> impl std::future::Future<Output = Result<Blob<T>, Self::Err>>
    where
        T: BlobSchema,
    {
        async move {
            let path = self.prefix.child(hex::encode(handle.bytes));
            let result = self.store.get(&path).await?;
            let object = result.bytes().await?;
            let bytes: Bytes = object.into();
            Ok(Blob::new(bytes))
        }
    }
}

impl<H> Push<H> for ObjectRepo<H>
where
    H: HashProtocol,
{
    type Err = object_store::Error;

    fn push<T>(
        &self,
        blob: Blob<T>,
    ) -> impl std::future::Future<Output = Result<Value<Handle<H, T>>, Self::Err>>
    where
        T: BlobSchema,
        Handle<H, T>: ValueSchema,
    {
        async move {
            let handle = blob.get_handle();
            let path = self.prefix.child(hex::encode(handle.bytes));
            let put_result = self
                .store
                .put_opts(
                    &path,
                    bytes::Bytes::copy_from_slice(&blob.bytes).into(), // This copy could be avoided if bytes::Bytes was open...
                    PutMode::Create.into(),
                )
                .await;
            match put_result {
                Ok(_) | Err(object_store::Error::AlreadyExists { .. }) => Ok(handle),
                Err(e) => Err(e),
            }
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
    H: HashProtocol,
{
    type CheckoutErr = CheckoutErr;
    type CommitErr = CommitErr;

    async fn checkout(&self) -> Result<Option<Value<Hash<H>>>, Self::CheckoutErr> {
        let result = self.store.get(&self.path).await;
        match result {
            Ok(result) => {
                let bytes = result.bytes().await?;
                let value = (&bytes[..]).try_into()?;
                Ok(Some(Value::new(value)))
            }
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(e)?,
        }
    }

    async fn commit(
        &self,
        old_hash: Option<Value<Hash<H>>>,
        new_hash: Value<Hash<H>>,
    ) -> Result<CommitResult<H>, Self::CommitErr> {
        let new_bytes = bytes::Bytes::copy_from_slice(&new_hash.bytes);

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
                        let stored_hash = Value::new(stored_value);
                        if old_hash != stored_hash {
                            return Ok(CommitResult::Conflict(Some(stored_hash)));
                        }
                        match self
                            .store
                            .put_opts(
                                &self.path,
                                new_bytes.clone().into(),
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
                    .put_opts(&self.path, new_bytes.clone().into(), PutMode::Create.into())
                    .await
                {
                    Ok(_) => return Ok(CommitResult::Success()), // Successfully committed
                    Err(object_store::Error::AlreadyExists { .. }) => {
                        let result = self.store.get(&self.path).await;
                        match result {
                            Ok(result) => {
                                let stored_bytes = result.bytes().await?;
                                let stored_value = (&stored_bytes[..]).try_into()?;
                                return Ok(CommitResult::Conflict(Some(Value::new(stored_value))));
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
