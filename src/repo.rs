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
pub mod branch;
pub mod commit;
pub mod objectstore;
pub mod pile;

use std::{
    convert::Infallible,
    error::Error,
    fmt::{self, Debug},
};

use commit::commit;
use ed25519_dalek::SigningKey;
use futures::{stream, Stream, StreamExt};
use itertools::Itertools;

use crate::prelude::valueschemas::GenId;
use crate::repo::branch::branch;
use crate::{
    and,
    blob::{schemas::UnknownBlob, Blob, BlobSchema, BlobSet, ToBlob},
    find,
    id::Id,
    metadata::metadata,
    trible::TribleSet,
    value::{
        schemas::hash::{Handle, HashProtocol},
        Value, ValueSchema,
    },
    NS,
};

use crate::{
    blob::schemas::simplearchive::SimpleArchive,
    value::schemas::{ed25519 as ed, hash::Blake3, shortstring::ShortString},
};

NS! {
    /// The `commits` namespace contains attributes describing commits in a repository.
    /// Commits are a fundamental building block of version control systems.
    /// They represent a snapshot of the repository at a specific point in time.
    /// Commits are immutable, append-only, and form a chain of history.
    /// Each commit is identified by a unique hash, and contains a reference to the previous commit.
    /// Commits are signed by the author, and can be verified by anyone with the author's public key.
    pub namespace repo {
        /// The actual data of the commit.
        "4DD4DDD05CC31734B03ABB4E43188B1F" as content: Handle<Blake3, SimpleArchive>;
        /// A commit that this commit is based on.
        "317044B612C690000D798CA660ECFD2A" as parent: Handle<Blake3, SimpleArchive>;
        /// A short message describing the commit.
        /// Used by tools displaying the commit history.
        "12290C0BE0E9207E324F24DDE0D89300" as short_message: ShortString;
        /// The hash of the first commit in the commit chain of the branch.
        "272FBC56108F336C4D2E17289468C35F" as head: Handle<Blake3, SimpleArchive>;
        /// An id used to track the branch.
        /// This id is unique to the branch, and is used to identify the branch in the repository.
        "8694CC73AF96A5E1C7635C677D1B928A" as branch: GenId;
        //"723C45065E7FCF1D52E86AD8D856A20D" as cached_rollup: Handle<Blake3, SuccinctArchive<CachedUniverse<1024, 1024, CompressedUniverse<DacsOpt>>, Rank9Sel>>;
        /// The author of the signature identified by their ed25519 public key.
        "ADB4FFAD247C886848161297EFF5A05B" as signed_by: ed::ED25519PublicKey;
        /// The `r` part of a ed25519 signature.
        "9DF34F84959928F93A3C40AEB6E9E499" as signature_r: ed::ED25519RComponent;
        /// The `s` part of a ed25519 signature.
        "1ACE03BF70242B289FDF00E4327C3BC6" as signature_s: ed::ED25519SComponent;
    }
}

pub trait ListBlobs<H: HashProtocol> {
    type Err;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>>;
}
pub trait GetBlob<H: HashProtocol> {
    type Err;

    fn get<T>(
        &self,
        handle: Value<Handle<H, T>>,
    ) -> impl std::future::Future<Output = Result<Blob<T>, Self::Err>>
    where
        T: BlobSchema + 'static;
}

pub trait PutBlob<H> {
    type Err;

    fn put<T>(
        &self,
        blob: Blob<T>,
    ) -> impl std::future::Future<Output = Result<Value<Handle<H, T>>, Self::Err>>
    where
        T: BlobSchema + 'static,
        Handle<H, T>: ValueSchema;
}

pub trait BlobRepo<H: HashProtocol>: ListBlobs<H> + GetBlob<H> + PutBlob<H> {}

pub trait Repo<H: HashProtocol>: BlobRepo<H> + BranchRepo<H> {}

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

impl<H> GetBlob<H> for BlobSet<H>
where
    H: HashProtocol,
{
    type Err = NotFoundErr;

    async fn get<T>(&self, handle: Value<Handle<H, T>>) -> Result<Blob<T>, Self::Err>
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
    Conflict(Option<Value<Handle<H, SimpleArchive>>>),
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
    ) -> impl std::future::Future<Output = Result<Option<Value<Handle<H, SimpleArchive>>>, Self::Err>>;
}

pub trait PushBranch<H: HashProtocol> {
    type Err;

    fn push(
        &self,
        id: Id,
        old: Option<Value<Handle<H, SimpleArchive>>>,
        new: Value<Handle<H, SimpleArchive>>,
    ) -> impl std::future::Future<Output = Result<PushResult<H>, Self::Err>>;
}

pub trait BranchRepo<H: HashProtocol>: ListBranches<H> + PullBranch<H> + PushBranch<H> {}

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
            <BS as GetBlob<HS>>::Err,
            <BT as PutBlob<HT>>::Err,
        >,
    >,
> + 'a
where
    BS: ListBlobs<HS> + GetBlob<HS>,
    BT: PutBlob<HT>,
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
                    .get(source_handle)
                    .await
                    .map_err(|e| TransferError::Load(e))?;
                let target_handle = target
                    .put(blob)
                    .await
                    .map_err(|e| TransferError::Store(e))?;
                Ok((source_handle, target_handle))
            },
        );
    r
}

pub struct MergeError();

/// Merges the contents of a source branch into a target branch.
/// The merge is performed by creating a new merge commit that has both the source and target branch as parents.
/// The target branch is then updated to point to the new merge commit.
pub async fn merge(
    signing_key: SigningKey,
    msg: Option<&str>,
    source: &impl Repo<Blake3>,
    target: &impl Repo<Blake3>,
    source_branch: Id,
    target_branch: Id,
) -> Result<(), MergeError> {
    let Ok(mut old_target_branch) = target.pull(target_branch).await else {
        return Err(MergeError());
    };

    loop {
        let Ok(source_branch_handle) = source.pull(source_branch).await else {
            return Err(MergeError());
        };

        let Some(target_branch_handle) = old_target_branch else {
            return Err(MergeError());
        };

        let Some(source_branch_handle) = source_branch_handle else {
            return Err(MergeError());
        };

        let Ok(source_branch_blob) = source.get(source_branch_handle).await else {
            return Err(MergeError());
        };

        let Ok(target_branch_blob) = target.get(target_branch_handle).await else {
            return Err(MergeError());
        };

        let Ok(source_branch_set): Result<TribleSet, _> = source_branch_blob.try_from_blob() else {
            return Err(MergeError());
        };

        let Ok(target_branch_set): Result<TribleSet, _> = target_branch_blob.try_from_blob() else {
            return Err(MergeError());
        };

        let source_head = match find!(
            (head: Value<_>),
            repo::pattern!(&source_branch_set, [{ head: head }])
        )
        .at_most_one()
        {
            Ok(Some((result,))) => result,
            Ok(None) => return Err(MergeError()),
            Err(_) => return Err(MergeError()),
        };

        let (target_head, target_name) = match find!(
            (head: Value<_>, name: Value<_>),
            and!{
                repo::pattern!(&target_branch_set, [{ head: head }]),
                metadata::pattern!(&target_branch_set, [{ name: name }])
            }
        )
        .at_most_one()
        {
            Ok(Some(result)) => result,
            Ok(None) => return Err(MergeError()),
            Err(_) => return Err(MergeError()),
        };

        let parents = [source_head, target_head];

        let commit = commit(&signing_key, parents, msg, None).to_blob();

        let branch = branch(
            &signing_key,
            target_branch,
            target_name.from_value(),
            commit.clone(),
        )
        .to_blob();

        let Ok(_) = target.put(commit).await else {
            return Err(MergeError());
        };

        let Ok(branch_handle) = target.put(branch).await else {
            return Err(MergeError());
        };

        match target
            .push(target_branch, Some(target_branch_handle), branch_handle)
            .await
        {
            Ok(PushResult::Success()) => return Ok(()),
            Ok(PushResult::Conflict(conflicting_handle)) =>  {
                old_target_branch = conflicting_handle;
                continue;
            },
            Err(_) => return Err(MergeError()),
        };
    }
}
