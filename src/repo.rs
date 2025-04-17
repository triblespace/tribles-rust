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

/// The `ListBlobs` trait is used to list all blobs in a repository.
pub trait ListBlobs<H: HashProtocol> {
    type Err: std::error::Error;

    /// Lists all blobs in the repository.
    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>>;
}

/// The `GetBlob` trait is used to retrieve blobs from a repository.
pub trait GetBlob<H: HashProtocol> {
    type Err: std::error::Error;

    /// Retrieves a blob from the repository by its handle.
    /// The handle is a unique identifier for the blob, and is used to retrieve it from the repository.
    /// The blob is returned as a `Blob` object, which contains the raw bytes of the blob,
    /// which can be deserialized via the appropriate schema type, which is specified by the `T` type parameter.
    ///
    /// # Errors
    /// Returns an error if the blob could not be found in the repository.
    /// The error type is specified by the `Err` associated type.
    fn get<T>(
        &self,
        handle: Value<Handle<H, T>>,
    ) -> impl std::future::Future<Output = Result<Blob<T>, Self::Err>>
    where
        T: BlobSchema + 'static;
}

/// The `PutBlob` trait is used to store blobs in a repository.
pub trait PutBlob<H> {
    type Err: std::error::Error;

    fn put<T>(
        &self,
        blob: Blob<T>,
    ) -> impl std::future::Future<Output = Result<Value<Handle<H, T>>, Self::Err>>
    where
        T: BlobSchema + 'static,
        Handle<H, T>: ValueSchema;
}

pub trait BlobRepo<H: HashProtocol>: ListBlobs<H> + GetBlob<H> + PutBlob<H> {}

#[derive(Debug)]
pub struct MergeError();

pub struct Repo<H: HashProtocol, Blobs: BlobRepo<H>, Branches: BranchRepo<H>> {
    blobs: Blobs,
    branches: Branches,
    hash_protocol: H,
}

impl<H, Blobs, Branches> Repo<H, Blobs, Branches>
where
    H: HashProtocol,
    Blobs: BlobRepo<H>,
    Branches: BranchRepo<H> {
    async fn commit(
        &self,
        branch_id: Id,
        msg: Option<&str>,
        content: TribleSet,
        commit_signing_key: SigningKey,
        branch_signing_key: SigningKey,
    ) {
        let mut current_branch_handle = self
            .branches
            .pull(branch_id)
            .await
            .expect("failed to pull branch")
            .expect("branch not found");

        loop {
            let current_branch_blob = self
                .blobs
                .get(current_branch_handle)
                .await
                .expect("failed to get current head blob");
            let current_head: TribleSet = current_branch_blob
                .try_from_blob()
                .expect("failed to convert blob");
            let (parent,) = find!(
                (head: Value<_>),
                repo::pattern!(&current_head, [{ head: head }])
            ).exactly_one().expect("failed to find head");

            let content: Blob<SimpleArchive> = content.to_blob();
            let content_put_progress = self
                .blobs
                .put(content.clone());

            let commit = commit(&commit_signing_key, [parent], msg, Some(content)).to_blob();
            let commit_put_progress = self.blobs.put(commit.clone());

            let (branch_name,) = find!(
                (name: Value<_>),
                metadata::pattern!(&current_head, [{ name: name }])
            ).exactly_one().expect("failed to find branch name");

            let branch = branch(&branch_signing_key, branch_id, branch_name.from_value(), commit).to_blob();
            let branch_put_progres = self.blobs.put(branch);

            content_put_progress.await.expect("failed to put content");
            commit_put_progress.await.expect("failed to put commit");
            let branch_handle = branch_put_progres.await.expect("failed to put branch");

            let push_result = self
                .branches
                .push(branch_id, Some(current_branch_handle), branch_handle)
                .await
                .expect("failed to push branch");

            
            current_branch_handle = match push_result {
                PushResult::Success() => return,
                PushResult::Conflict(conflicting_handle) => conflicting_handle.expect("branch doesn't exist"),
            }
        }
    }

    /// Merges the contents of a source branch into a target branch.
    /// The merge is performed by creating a new merge commit that has both the source and target branch as parents.
    /// The target branch is then updated to point to the new merge commit.
    async fn merge<OtherBlobs, OtherBranches>(
        &self,
        self_branch: Id,
        source: Repo<H, OtherBlobs, OtherBranches>,
        source_branch: Id,
        msg: Option<&str>,
        commit_signing_key: SigningKey,
        branch_signing_key: SigningKey,
    ) -> Result<(), MergeError>
    where OtherBlobs: BlobRepo<H>,
          OtherBranches: BranchRepo<H>,{
        let Ok(mut old_target_branch) = self.branches.pull(self_branch).await else {
            return Err(MergeError());
        };

        loop {
            let Ok(source_branch_handle) = source.branches.pull(source_branch).await else {
                return Err(MergeError());
            };

            let Some(target_branch_handle) = old_target_branch else {
                return Err(MergeError());
            };

            let Some(source_branch_handle) = source_branch_handle else {
                return Err(MergeError());
            };

            let Ok(source_branch_blob) = source.blobs.get(source_branch_handle).await else {
                return Err(MergeError());
            };

            let Ok(target_branch_blob) = self.blobs.get(target_branch_handle).await else {
                return Err(MergeError());
            };

            let Ok(source_branch_set): Result<TribleSet, _> = source_branch_blob.try_from_blob()
            else {
                return Err(MergeError());
            };

            let Ok(target_branch_set): Result<TribleSet, _> = target_branch_blob.try_from_blob()
            else {
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

            let commit = commit(&commit_signing_key, parents, msg, None).to_blob();

            let branch = branch(
                &branch_signing_key,
                self_branch,
                target_name.from_value(),
                commit.clone(),
            )
            .to_blob();

            let Ok(_) = self.blobs.put(commit).await else {
                return Err(MergeError());
            };

            let Ok(branch_handle) = self.blobs.put(branch).await else {
                return Err(MergeError());
            };

            match self
                .branches
                .push(self_branch, Some(target_branch_handle), branch_handle)
                .await
            {
                Ok(PushResult::Success()) => return Ok(()),
                Ok(PushResult::Conflict(conflicting_handle)) => {
                    old_target_branch = conflicting_handle;
                    continue;
                }
                Err(_) => return Err(MergeError()),
            };
        }
    }
}

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

/// The `ListBranches` trait is used to list all branches in a repository.
pub trait ListBranches<H: HashProtocol> {
    type Err: std::error::Error;

    /// Lists all branches in the repository.
    fn list<'a>(&'a self) -> impl Stream<Item = Result<Id, Self::Err>>;
}

/// The `PullBranch` trait is used to retrieve a branch from a repository.
pub trait PullBranch<H: HashProtocol> {
    type Err: std::error::Error;

    /// Retrieves a branch from the repository by its id.
    fn pull(
        &self,
        id: Id,
    ) -> impl std::future::Future<Output = Result<Option<Value<Handle<H, SimpleArchive>>>, Self::Err>>;
}

pub trait PushBranch<H: HashProtocol> {
    type Err: std::error::Error;
    /// Pushes a branch to the repository, creating or updating it.
    ///
    /// # Parameters
    /// * `old` - Expected current value of the branch (None if creating new)
    /// * `new` - Value to update the branch to
    ///
    /// # Returns
    /// * `Success` - Push completed successfully
    /// * `Conflict(current)` - Failed because the branch's current value doesn't match `old`
    ///   (contains the actual current value for conflict resolution)
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
