//! This module provides a high-level API for storing and retrieving data from repositories.
//! The design is inspired by Git, but with a focus on object/content-addressed storage.
//! It separates storage concerns from the data model, and reduces the mutable state of the repository,
//! to an absolute minimum, making it easier to reason about and allowing for different storage backends.
//!
//! Blob repositories are collections of blobs that can be content-addressed by their hash.
//! This is typically local `.pile` file or a S3 bucket or a similar service.
//! On their own they have no notion of branches or commits, or other stateful constructs.
//! As such they also don't have a notion of time, order or history,
//! massively relaxing the constraints on storage.
//! This makes it possible to use a wide range of storage services, including those that don't support
//! atomic transactions or have other limitations.
//!
//! Branch repositories on the other hand are a stateful construct that can be used to represent a branch pointing to a specific commit.
//! They are stored in a separate repository, typically a  local `.pile` file, a database or an S3 compatible service with a compare-and-swap operation,
//! and can be used to represent the state of a repository at a specific point in time.
//!
//! Technically, branches are just a mapping from a branch id to a blob hash,
//! But because TribleSets are themselves easily stored in a blob, and because
//! trible commit histories are an append-only chain of TribleSet metadata,
//! the hash of the head is sufficient to represent the entire history of a branch.
//!
//!
pub mod commit;
pub mod objectstore;
pub mod pile;

use std::{
    convert::Infallible,
    error::Error,
    fmt::{self, Debug},
};

use futures::{stream, Stream, StreamExt};

use crate::{
    blob::{schemas::UnknownBlob, Blob, BlobSchema, BlobSet},
    id::Id,
    value::{
        schemas::hash::{Handle, Hash, HashProtocol},
        Value, ValueSchema,
    },
};

#[derive(Debug)]
pub enum TransferError<ListErr, LoadErr, StoreErr> {
    List(ListErr),
    Load(LoadErr),
    Store(StoreErr),
}

impl<ListErr, LoadErr, StoreErr> fmt::Display for TransferError<ListErr, LoadErr, StoreErr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to transfer blob")
    }
}

impl<ListErr, LoadErr, StoreErr> Error for TransferError<ListErr, LoadErr, StoreErr>
where
    ListErr: Debug + Error + 'static,
    LoadErr: Debug + Error + 'static,
    StoreErr: Debug + Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::List(e) => Some(e),
            Self::Load(e) => Some(e),
            Self::Store(e) => Some(e),
        }
    }
}

pub async fn transfer<'a, BS, BT, HS, HT, S>(
    source: &'a BS,
    target: &'a BT,
) -> impl Stream<
    Item = Result<
        (
            Value<Handle<HS, UnknownBlob>>,
            Value<Handle<HT, UnknownBlob>>,
        ),
        TransferError<
            <BS as ListBlobs<HS>>::Err,
            <BS as PullBlob<HS>>::Err,
            <BT as PushBlob<HT>>::Err,
        >,
    >,
> + 'a
where
    BS: ListBlobs<HS> + PullBlob<HS>,
    BT: PushBlob<HT>,
    HS: 'static + HashProtocol,
    HT: 'static + HashProtocol,
{
    let l = source.list();
    let r =
        l.then(
            move |source_handle: Result<
                Value<Handle<HS, UnknownBlob>>,
                <BS as ListBlobs<HS>>::Err,
            >| async move {
                let source_handle = source_handle.map_err(|e| TransferError::List(e))?;
                let blob = source
                    .pull(source_handle)
                    .await
                    .map_err(|e| TransferError::Load(e))?;
                let target_handle = target
                    .push(blob)
                    .await
                    .map_err(|e| TransferError::Store(e))?;
                Ok((source_handle, target_handle))
            },
        );
    r
}

pub trait ListBlobs<H: HashProtocol> {
    type Err;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>>;
}
pub trait PullBlob<H: HashProtocol> {
    type Err;

    fn pull<T>(
        &self,
        handle: Value<Handle<H, T>>,
    ) -> impl std::future::Future<Output = Result<Blob<T>, Self::Err>>
    where
        T: BlobSchema + 'static;
}

pub trait PushBlob<H> {
    type Err;

    fn push<T>(
        &self,
        blob: Blob<T>,
    ) -> impl std::future::Future<Output = Result<Value<Handle<H, T>>, Self::Err>>
    where
        T: BlobSchema + 'static,
        Handle<H, T>: ValueSchema;
}

pub trait BlobRepo<H: HashProtocol>: ListBlobs<H> + PullBlob<H> + PushBlob<H> {}

#[derive(Debug)]
pub struct NotFoundErr();

impl fmt::Display for NotFoundErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no blob for hash in blobset")
    }
}

impl Error for NotFoundErr {}

impl<H> ListBlobs<H> for BlobSet<H>
where
    H: HashProtocol,
{
    type Err = Infallible;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>> {
        stream::iter((&self).into_iter().map(|(&hash, _)| Ok(hash)))
    }
}

impl<H> PullBlob<H> for BlobSet<H>
where
    H: HashProtocol,
{
    type Err = NotFoundErr;

    async fn pull<T>(&self, handle: Value<Handle<H, T>>) -> Result<Blob<T>, Self::Err>
    where
        T: BlobSchema,
    {
        self.get(handle).ok_or(NotFoundErr())
    }
}

#[derive(Debug)]
pub enum PushResult<H>
where
    H: HashProtocol,
{
    Success(),
    Conflict(Option<Value<Hash<H>>>),
}

pub trait ListBranches<H: HashProtocol> {
    type Err;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Id, Self::Err>>;
}

pub trait PullBranch<H: HashProtocol> {
    type Err;

    fn pull(
        &self,
        id: Id,
    ) -> impl std::future::Future<Output = Result<Option<Value<Hash<H>>>, Self::Err>>;
}

pub trait PushBranch<H: HashProtocol> {
    type Err;

    fn push(
        &self,
        id: Id,
        old: Option<Value<Hash<H>>>,
        new: Value<Hash<H>>,
    ) -> impl std::future::Future<Output = Result<PushResult<H>, Self::Err>>;
}

pub trait BranchRepo<H: HashProtocol>: ListBranches<H> + PullBranch<H> + PushBranch<H> {}
