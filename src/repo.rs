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
//pub mod objectstore;
pub mod pile;

use std::{
    error::Error,
    fmt::{self, Debug},
};

use commit::commit;
use ed25519_dalek::SigningKey;
use itertools::Itertools;

use crate::repo::branch::branch;
use crate::{
    and,
    blob::{schemas::UnknownBlob, Blob, BlobSchema, ToBlob},
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
use crate::{id::ufoid, prelude::valueschemas::GenId};

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
pub trait BlobStoreListOp<H: HashProtocol> {
    type Iter<'a>: Iterator<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>> where Self: 'a;
    type Err: Error + Debug + Send + Sync + 'static;

    /// Lists all blobs in the repository.
    fn list<'a>(&'a self)
        -> Self::Iter<'a>;
}

/// The `GetBlob` trait is used to retrieve blobs from a repository.
pub trait BlobStoreGetOp<H: HashProtocol> {
    type Err: Error + Debug + Send + Sync + 'static;

    /// Retrieves a blob from the repository by its handle.
    /// The handle is a unique identifier for the blob, and is used to retrieve it from the repository.
    /// The blob is returned as a `Blob` object, which contains the raw bytes of the blob,
    /// which can be deserialized via the appropriate schema type, which is specified by the `T` type parameter.
    ///
    /// # Errors
    /// Returns an error if the blob could not be found in the repository.
    /// The error type is specified by the `Err` associated type.
    fn get<T>(&self, handle: Value<Handle<H, T>>) -> Result<Blob<T>, Self::Err>
    where
        T: BlobSchema + 'static;
}

/// The `PutBlob` trait is used to store blobs in a repository.
pub trait BlobStorePutOp<H> {
    type Err: Error + Debug + Send + Sync + 'static;

    fn put<T>(&mut self, blob: Blob<T>) -> Result<Value<Handle<H, T>>, Self::Err>
    where
        T: BlobSchema + 'static,
        Handle<H, T>: ValueSchema;
}

pub trait BlobStorage<H: HashProtocol>:
    BlobStoreListOp<H> + BlobStoreGetOp<H> + BlobStorePutOp<H>
{
}

#[derive(Debug)]
pub enum PushResult<H>
where
    H: HashProtocol,
{
    Success(),
    Conflict(Option<Value<Handle<H, SimpleArchive>>>),
}

pub trait BranchRepo<H: HashProtocol> {
    type ListErr: Error + Debug + Send + Sync + 'static;
    type PullErr: Error + Debug + Send + Sync + 'static;
    type PushErr: Error + Debug + Send + Sync + 'static;

    type ListIter<'a>: Iterator<Item = Result<Id, Self::ListErr>> where Self: 'a;

    /// Lists all branches in the repository.
    /// This function returns a stream of branch ids.
    fn list<'a>(&'a self) -> Self::ListIter<'a>;

    /// Retrieves a branch from the repository by its id.
    /// The id is a unique identifier for the branch, and is used to retrieve it from the repository.
    ///
    /// # Errors
    /// Returns an error if the branch could not be found in the repository.
    ///
    /// # Parameters
    /// * `id` - The id of the branch to retrieve.
    ///
    /// # Returns
    /// * A future that resolves to the handle of the branch.
    /// * The handle is a unique identifier for the branch, and is used to retrieve it from the repository.
    fn pull(&self, id: Id) -> Result<Option<Value<Handle<H, SimpleArchive>>>, Self::PullErr>;

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
        &mut self,
        id: Id,
        old: Option<Value<Handle<H, SimpleArchive>>>,
        new: Value<Handle<H, SimpleArchive>>,
    ) -> Result<PushResult<H>, Self::PushErr>;
}

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

pub fn transfer<'a, BS, BT, HS, HT, S>(
    source: &'a BS,
    target: &'a mut BT,
) -> impl Iterator<
    Item = Result<
        (
            Value<Handle<HS, UnknownBlob>>,
            Value<Handle<HT, UnknownBlob>>,
        ),
        TransferError<
            <BS as BlobStoreListOp<HS>>::Err,
            <BS as BlobStoreGetOp<HS>>::Err,
            <BT as BlobStorePutOp<HT>>::Err,
        >,
    >,
> + 'a
where
    BS: BlobStoreListOp<HS> + BlobStoreGetOp<HS>,
    BT: BlobStorePutOp<HT>,
    HS: 'static + HashProtocol,
    HT: 'static + HashProtocol,
{
    source.list().map(
        move |source_handle: Result<
            Value<Handle<HS, UnknownBlob>>,
            <BS as BlobStoreListOp<HS>>::Err,
        >| {
            let source_handle = source_handle.map_err(|e| TransferError::List(e))?;
            let blob = source
                .get(source_handle)
                .map_err(|e| TransferError::Load(e))?;
            let target_handle = target.put(blob).map_err(|e| TransferError::Store(e))?;
            Ok((source_handle, target_handle))
        },
    )
}

/// An error that can occur when creating a commit.
/// This error can be caused by a failure to store the content or metadata blobs.
#[derive(Debug)]
pub enum CreateCommitError<BlobErr: Error + Debug + Send + Sync + 'static> {
    /// Failed to store the content blob.
    ContentStorageError(BlobErr),
    /// Failed to store the commit metadata blob.
    CommitStorageError(BlobErr),
}

impl<BlobErr: Error + Debug + Send + Sync + 'static> fmt::Display for CreateCommitError<BlobErr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateCommitError::ContentStorageError(e) => write!(f, "Content storage failed: {}", e),
            CreateCommitError::CommitStorageError(e) => {
                write!(f, "Commit metadata storage failed: {}", e)
            }
        }
    }
}

impl<BlobErr: Error + Debug + Send + Sync + 'static> Error for CreateCommitError<BlobErr> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CreateCommitError::ContentStorageError(e) => Some(e),
            CreateCommitError::CommitStorageError(e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub struct MergeError();

pub struct Repo<Blobs: BlobStorage<Blake3>, Branches: BranchRepo<Blake3>> {
    blobs: Blobs,
    branches: Branches,
}

impl<Blobs, Branches> Repo<Blobs, Branches>
where
    Blobs: BlobStorage<Blake3>,
    Branches: BranchRepo<Blake3>,
{
    /// Creates a new repository with the given blob and branch repositories.
    /// The blob repository is used to store the actual data of the repository,
    /// while the branch repository is used to store the state of the repository.
    /// The hash protocol is used to hash the blobs and branches in the repository.
    ///
    /// # Parameters
    /// * `blobs` - The blob repository to use for storing blobs.
    /// * `branches` - The branch repository to use for storing branches.
    /// # Returns
    /// * A new `Repo` object that can be used to store and retrieve blobs and branches.
    pub fn new(blobs: Blobs, branches: Branches) -> Self {
        Self { blobs, branches }
    }

    /// Creates an immutable commit object and stores it (and its content, if provided)
    /// in the blob repository. Does not update any branches.
    ///
    /// The content blob itself is passed in, as it's needed for signing.
    /// This function handles storing both the content blob (if provided) and the commit metadata blob.
    ///
    /// # Errors
    /// Returns `CreateCommitError::ContentStorageError` if storing the content blob fails.
    /// Returns `CreateCommitError::CommitStorageError` if storing the commit metadata blob fails.
    pub fn create_commit(
        &mut self,
        parents: impl IntoIterator<Item = Value<Handle<Blake3, SimpleArchive>>>,
        // Accept the actual content Blob, not just the handle
        content_blob: Option<Blob<SimpleArchive>>,
        msg: Option<&str>,
        commit_signing_key: &SigningKey,
    ) -> Result<
        Value<Handle<Blake3, SimpleArchive>>,
        CreateCommitError<<Blobs as BlobStorePutOp<Blake3>>::Err>,
    > {
        // 1. Store content blob first (if it exists)
        //    We need to clone it if commit() takes ownership, or ensure commit() borrows.
        //    Assuming commit() takes Option<Blob<SimpleArchive>> by value (ownership).
        if let Some(blob) = &content_blob {
            self.blobs
                .put(blob.clone())
                .map_err(CreateCommitError::ContentStorageError)?;
            // We ignore the handle returned here, as commit() likely recalculates or doesn't need it explicitly.
        }

        // 2. Create the commit TribleSet using the lower-level function from commit.rs
        //    Pass the actual content_blob for signing.
        let commit_set =
            crate::repo::commit::commit(commit_signing_key, parents, msg, content_blob); // Pass blob ownership

        // 3. Store the commit metadata blob
        let commit_meta_blob = commit_set.to_blob(); // This doesn't fail
        self.blobs
            .put(commit_meta_blob)
            .map_err(CreateCommitError::CommitStorageError) // Map error to the correct variant
    }

    /// Initializes a new branch in the repository.
    /// Branches are the only mutable state in the repository,
    /// and are used to represent the state of a commit chain at a specific point in time.
    /// A branch must always point to a commit, and this function can be used to create a new branch.

    /// Creates a new branch in the repository.
    /// This branch is a pointer to a specific commit in the repository.
    /// The branch is created with name and is initialized to point to the opionally given commit.
    /// The branch is signed by the branch signing key.
    ///
    /// # Parameters
    /// * `branch_name` - The name of the branch to create.
    /// * `commit` - The handle referencing the commit to initialize the branch to.
    /// * `branch_signing_key` - The signing key to use for signing the branch.
    /// # Returns
    /// * A future that resolves to the id of the new branch.
    ///
    pub fn branch(
        &mut self,
        branch_name: &str,
        commit: Value<Handle<Blake3, SimpleArchive>>,
        branch_signing_key: SigningKey,
    ) -> Id {
        let branch_id = *ufoid();
        let commit_blob = self.blobs.get(commit).expect("failed to get commit blob");

        let branch = branch(&branch_signing_key, branch_id, branch_name, commit_blob);

        let branch_blob = branch.to_blob();
        let branch_handle = self
            .blobs
            .put(branch_blob)
            .expect("failed to put branch blob");

        let push_result = self
            .branches
            .push(branch_id, None, branch_handle)
            .expect("failed to push branch");

        match push_result {
            PushResult::Success() => branch_id,
            PushResult::Conflict(_) => panic!("branch already exists"),
        }
    }

    /// Commits the given content to the specified branch.
    /// This stores the following information in the repository:
    /// * A blob containing the content of the commit.
    /// * A commit blob that contains the commit message and a reference to the previous commit,
    /// signed by the commit signing key.
    /// * A branch blob that contains the branch id and a reference to the commit blob,
    /// signed by the branch signing key.
    /// * The branch is updated to point to the new commit.
    ///
    /// # Parameters
    /// * `branch_id` - The id of the branch to commit to.
    /// * `msg` - An optional commit message.
    /// * `content` - The content to commit.
    /// * `commit_signing_key` - The signing key to use for signing the commit.
    /// * `branch_signing_key` - The signing key to use for signing the branch.
    /// # Returns
    /// * A future that resolves when the commit is complete.
    fn commit(
        &mut self,
        branch_id: Id,
        msg: Option<&str>,
        content: TribleSet,
        commit_signing_key: SigningKey,
        branch_signing_key: SigningKey,
    ) {
        let mut current_branch_handle = self
            .branches
            .pull(branch_id)
            .expect("failed to pull branch")
            .expect("branch not found");

        loop {
            let current_branch_blob = self
                .blobs
                .get(current_branch_handle)
                .expect("failed to get current head blob");
            let current_head: TribleSet = current_branch_blob
                .try_from_blob()
                .expect("failed to convert blob");
            let (parent,) = find!(
                (head: Value<_>),
                repo::pattern!(&current_head, [{ head: head }])
            )
            .exactly_one()
            .expect("failed to find head");

            let content: Blob<SimpleArchive> = content.to_blob();
            let content_put_progress = self.blobs.put(content.clone());

            let commit = commit(&commit_signing_key, [parent], msg, Some(content)).to_blob();
            let commit_put_progress = self.blobs.put(commit.clone());

            let (branch_name,) = find!(
                (name: Value<_>),
                metadata::pattern!(&current_head, [{ name: name }])
            )
            .exactly_one()
            .expect("failed to find branch name");

            let branch = branch(
                &branch_signing_key,
                branch_id,
                branch_name.from_value(),
                commit,
            )
            .to_blob();
            let branch_put_progres = self.blobs.put(branch);

            content_put_progress.expect("failed to put content");
            commit_put_progress.expect("failed to put commit");
            let branch_handle = branch_put_progres.expect("failed to put branch");

            let push_result = self
                .branches
                .push(branch_id, Some(current_branch_handle), branch_handle)
                .expect("failed to push branch");

            current_branch_handle = match push_result {
                PushResult::Success() => return,
                PushResult::Conflict(conflicting_handle) => {
                    conflicting_handle.expect("branch doesn't exist")
                }
            }
        }
    }

    /// Merges the contents of a source branch into a target branch.
    /// The merge is performed by creating a new merge commit that has both the source and target branch as parents.
    /// The target branch is then updated to point to the new merge commit.
    fn merge<OtherBlobs, OtherBranches>(
        &mut self,
        self_branch: Id,
        source: Repo<OtherBlobs, OtherBranches>,
        source_branch: Id,
        msg: Option<&str>,
        commit_signing_key: SigningKey,
        branch_signing_key: SigningKey,
    ) -> Result<(), MergeError>
    where
        OtherBlobs: BlobStorage<Blake3>,
        OtherBranches: BranchRepo<Blake3>,
    {
        let Ok(mut old_target_branch) = self.branches.pull(self_branch) else {
            return Err(MergeError());
        };

        loop {
            let Ok(source_branch_handle) = source.branches.pull(source_branch) else {
                return Err(MergeError());
            };

            let Some(target_branch_handle) = old_target_branch else {
                return Err(MergeError());
            };

            let Some(source_branch_handle) = source_branch_handle else {
                return Err(MergeError());
            };

            let Ok(source_branch_blob) = source.blobs.get(source_branch_handle) else {
                return Err(MergeError());
            };

            let Ok(target_branch_blob) = self.blobs.get(target_branch_handle) else {
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

            let Ok(_) = self.blobs.put(commit) else {
                return Err(MergeError());
            };

            let Ok(branch_handle) = self.blobs.put(branch) else {
                return Err(MergeError());
            };

            match self
                .branches
                .push(self_branch, Some(target_branch_handle), branch_handle)
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
