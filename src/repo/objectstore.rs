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
use crate::id::{Id, RawId};
use crate::value::{
    schemas::hash::{Handle, Hash, HashProtocol},
    RawValue, Value, ValueSchema,
};

use super::{ListBlobs, ListBranches, PullBlob, PullBranch, PushBlob, PushBranch, PushResult};

const BRANCH_INFIX: &str = "branches";
const BLOB_INFIX: &str = "blobs";

pub struct ObjectStoreRepo<H> {
    store: Box<dyn ObjectStore>,
    prefix: Path,
    _hasher: PhantomData<H>,
}

impl<H> ObjectStoreRepo<H> {
    pub fn with_url(url: &Url) -> Result<ObjectStoreRepo<H>, object_store::Error> {
        let (store, path) = parse_url(&url)?;
        Ok(ObjectStoreRepo {
            store,
            prefix: path,
            _hasher: PhantomData,
        })
    }
}

pub enum ListBlobsErr {
    List(object_store::Error),
    NotAFile(&'static str),
    BadNameHex(<RawValue as FromHex>::Error),
}

pub enum ListBranchesErr {
    List(object_store::Error),
    NotAFile(&'static str),
    BadNameHex(<RawId as FromHex>::Error),
    BadId,
}

impl<H> ListBlobs<H> for ObjectStoreRepo<H>
where
    H: HashProtocol,
{
    type Err = ListBlobsErr;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>> {
        self.store
            .list(Some(&self.prefix.child(BLOB_INFIX)))
            .map(|r| match r {
                Ok(meta) => {
                    let blob_name = meta
                        .location
                        .filename()
                        .ok_or(ListBlobsErr::NotAFile("no filename"))?;
                    let digest =
                        RawValue::from_hex(blob_name).map_err(|e| ListBlobsErr::BadNameHex(e))?;
                    Ok(Value::new(digest))
                }
                Err(e) => Err(ListBlobsErr::List(e)),
            })
            .boxed()
    }
}

impl<H> PullBlob<H> for ObjectStoreRepo<H>
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
            let path = self.prefix.child(BLOB_INFIX).child(hex::encode(handle.raw));
            let result = self.store.get(&path).await?;
            let object = result.bytes().await?;
            let bytes: Bytes = object.into();
            Ok(Blob::new(bytes))
        }
    }
}

impl<H> PushBlob<H> for ObjectStoreRepo<H>
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
            let path = self.prefix.child(BLOB_INFIX).child(hex::encode(handle.raw));
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

#[derive(Debug)]
pub enum PullBranchErr {
    ValidationErr(TryFromSliceError),
    StoreErr(object_store::Error),
}

impl fmt::Display for PullBranchErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::StoreErr(e) => write!(f, "checkout failed: {}", e),
            Self::ValidationErr(e) => write!(f, "checkout failed: {}", e),
        }
    }
}

impl Error for PullBranchErr {}

impl From<object_store::Error> for PullBranchErr {
    fn from(err: object_store::Error) -> Self {
        Self::StoreErr(err)
    }
}

impl From<TryFromSliceError> for PullBranchErr {
    fn from(err: TryFromSliceError) -> Self {
        Self::ValidationErr(err)
    }
}

#[derive(Debug)]
pub enum PushBranchErr {
    ValidationErr(TryFromSliceError),
    StoreErr(object_store::Error),
}

impl fmt::Display for PushBranchErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ValidationErr(e) => write!(f, "commit failed: {}", e),
            Self::StoreErr(e) => write!(f, "commit failed: {}", e),
        }
    }
}

impl Error for PushBranchErr {}

impl From<object_store::Error> for PushBranchErr {
    fn from(err: object_store::Error) -> Self {
        Self::StoreErr(err)
    }
}

impl From<TryFromSliceError> for PushBranchErr {
    fn from(err: TryFromSliceError) -> Self {
        Self::ValidationErr(err)
    }
}

impl<H> PullBranch<H> for ObjectStoreRepo<H>
where
    H: HashProtocol,
{
    type Err = PullBranchErr;

    async fn pull(&self, branch: Id) -> Result<Option<Value<Hash<H>>>, Self::Err> {
        let branch_name = hex::encode(&branch);
        let path = self.prefix.child(BRANCH_INFIX).child(branch_name);
        let result = self.store.get(&path).await;
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
}

impl<H> PushBranch<H> for ObjectStoreRepo<H>
where
    H: HashProtocol,
{
    type Err = PushBranchErr;

    async fn push(
        &self,
        branch_id: Id,
        old_hash: Option<Value<Hash<H>>>,
        new_hash: Value<Hash<H>>,
    ) -> Result<PushResult<H>, Self::Err> {
        let branch_name = hex::encode(&branch_id);
        let path = &self.prefix.child(BRANCH_INFIX).child(branch_name);

        let new_bytes = bytes::Bytes::copy_from_slice(&new_hash.raw);

        if let Some(old_hash) = old_hash {
            let mut result = self.store.get(path).await;
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
                            return Ok(PushResult::Conflict(Some(stored_hash)));
                        }
                        match self
                            .store
                            .put_opts(
                                &path,
                                new_bytes.clone().into(),
                                PutMode::Update(version).into(),
                            )
                            .await
                        {
                            Ok(_) => return Ok(PushResult::Success()), // Successfully committed
                            Err(object_store::Error::Precondition { .. }) => {
                                result = self.store.get(&path).await;
                                continue;
                            }
                            Err(e) => return Err(e.into()),
                        }
                    }
                    Err(object_store::Error::NotFound { .. }) => {
                        return Ok(PushResult::Conflict(None));
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        } else {
            loop {
                // Attempt to commit
                match self
                    .store
                    .put_opts(&path, new_bytes.clone().into(), PutMode::Create.into())
                    .await
                {
                    Ok(_) => return Ok(PushResult::Success()), // Successfully committed
                    Err(object_store::Error::AlreadyExists { .. }) => {
                        let result = self.store.get(path).await;
                        match result {
                            Ok(result) => {
                                let stored_bytes = result.bytes().await?;
                                let stored_value = (&stored_bytes[..]).try_into()?;
                                return Ok(PushResult::Conflict(Some(Value::new(stored_value))));
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

impl<H> ListBranches<H> for ObjectStoreRepo<H>
where
    H: HashProtocol,
{
    type Err = ListBranchesErr;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Id, Self::Err>> {
        self.store
            .list(Some(&self.prefix.child(BRANCH_INFIX)))
            .map(|r| match r {
                Ok(meta) => {
                    let branch_name = meta
                        .location
                        .filename()
                        .ok_or(ListBranchesErr::NotAFile("no filename"))?;
                    let digest =
                        RawId::from_hex(branch_name).map_err(|e| ListBranchesErr::BadNameHex(e))?;
                    let Some(new_id) = Id::new(digest) else {
                        return Err(ListBranchesErr::BadId);
                    };
                    Ok(new_id)
                }
                Err(e) => Err(ListBranchesErr::List(e)),
            })
            .boxed()
    }
}
