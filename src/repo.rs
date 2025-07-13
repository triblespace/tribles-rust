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
//! ## Basic usage
//!
//! ```rust,ignore
//! use ed25519_dalek::SigningKey;
//! use rand::rngs::OsRng;
//! use tribles::prelude::*;
//! use tribles::prelude::valueschemas::{GenId, ShortString};
//! use tribles::repo::{memoryrepo::MemoryRepo, Repository};
//!
//! let storage = MemoryRepo::default();
//! let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
//! let mut ws = repo.branch("main").expect("create branch");
//!
//! NS! {
//!     pub namespace literature {
//!         "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
//!         "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
//!         "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
//!     }
//! }
//! let author = fucid();
//! ws.commit(
//!     literature::entity!(&author, {
//!         firstname: "Frank",
//!         lastname: "Herbert",
//!     }),
//!     Some("initial commit"),
//! );
//!
//! match repo.push(&mut ws).expect("push") {
//!     None => {}
//!     Some(_) => panic!("unexpected conflict"),
//! }
//! ```
//!
//! `checkout` creates a new workspace from an existing branch while
//! `branch_from` can be used to start a new branch from a specific commit
//! handle. See `examples/workspace.rs` for a more complete example.
//!
//! ## Handling conflicts
//!
//! `push` may return `Some(conflict_ws)` when the branch has changed.
//! The returned workspace contains the updated branch metadata and must be
//! pushed after merging your changes:
//!
//! ```rust,ignore
//! while let Some(mut other) = repo.push(&mut ws)? {
//!     other.merge(&mut ws)?;
//!     ws = other;
//! }
//! ```
//!
//! `push` performs a compare‐and‐swap (CAS) update on the branch metadata.
//! This optimistic concurrency control keeps branches consistent without
//! locking and can be emulated by many storage systems (for example by
//! using conditional writes on S3).
//!
//! ## Git parallels
//!
//! The API deliberately mirrors concepts from Git to make its usage familiar:
//!
//! - A `Repository` stores commits and branch metadata similar to a remote.
//! - `Workspace` is akin to a working directory combined with an index. It
//!   tracks changes against a branch head until you `push` them.
//! - `branch` and `branch_from` correspond to creating new branches from the
//!   current head or from a specific commit, respectively.
//! - `push` updates the repository atomically. If the branch advanced in the
//!   meantime, you receive a conflict workspace which can be merged before
//!   retrying the push.
//! - `checkout` is similar to cloning a branch into a new workspace.
//!
//! `checkout` uses the repository's default signing key for new commits. If you
//! need to work with a different identity, the `_with_key` variants allow providing
//! an explicit key when branching or checking out.
//!
//! These parallels should help readers leverage their Git knowledge when
//! working with trible repositories.
//!
pub mod branch;
pub mod commit;
pub mod hybridstore;
pub mod memoryrepo;
pub mod objectstore;
pub mod pile;

use std::{
    convert::Infallible,
    error::Error,
    fmt::{self, Debug},
};

use commit::commit;
use itertools::Itertools;

use crate::{blob::MemoryBlobStore, repo::branch::branch};
use crate::{
    blob::{
        schemas::{simplearchive::UnarchiveError, UnknownBlob},
        Blob, BlobSchema, ToBlob, TryFromBlob,
    },
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
use ed25519_dalek::SigningKey;

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
pub trait BlobStoreList<H: HashProtocol> {
    type Iter<'a>: Iterator<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>>
    where
        Self: 'a;
    type Err: Error + Debug + Send + Sync + 'static;

    /// Lists all blobs in the repository.
    fn blobs<'a>(&'a self) -> Self::Iter<'a>;
}

/// The `GetBlob` trait is used to retrieve blobs from a repository.
pub trait BlobStoreGet<H: HashProtocol> {
    type GetError<E: std::error::Error>: Error;

    /// Retrieves a blob from the repository by its handle.
    /// The handle is a unique identifier for the blob, and is used to retrieve it from the repository.
    /// The blob is returned as a `Blob` object, which contains the raw bytes of the blob,
    /// which can be deserialized via the appropriate schema type, which is specified by the `T` type parameter.
    ///
    /// # Errors
    /// Returns an error if the blob could not be found in the repository.
    /// The error type is specified by the `Err` associated type.
    fn get<T, S>(
        &self,
        handle: Value<Handle<H, S>>,
    ) -> Result<T, Self::GetError<<T as TryFromBlob<S>>::Error>>
    where
        S: BlobSchema + 'static,
        T: TryFromBlob<S>,
        Handle<H, S>: ValueSchema;
}

/// The `PutBlob` trait is used to store blobs in a repository.
pub trait BlobStorePut<H: HashProtocol> {
    type PutError: Error + Debug + Send + Sync + 'static;

    fn put<S, T>(&mut self, item: T) -> Result<Value<Handle<H, S>>, Self::PutError>
    where
        S: BlobSchema + 'static,
        T: ToBlob<S>,
        Handle<H, S>: ValueSchema;
}

pub trait BlobStore<H: HashProtocol>: BlobStorePut<H> {
    type Reader: BlobStoreGet<H> + BlobStoreList<H> + Clone + Send + PartialEq + Eq + 'static;
    fn reader(&mut self) -> Self::Reader;
}

#[derive(Debug)]
pub enum PushResult<H>
where
    H: HashProtocol,
{
    Success(),
    Conflict(Option<Value<Handle<H, SimpleArchive>>>),
}

pub trait BranchStore<H: HashProtocol> {
    type BranchesError: Error + Debug + Send + Sync + 'static;
    type HeadError: Error + Debug + Send + Sync + 'static;
    type UpdateError: Error + Debug + Send + Sync + 'static;

    type ListIter<'a>: Iterator<Item = Result<Id, Self::BranchesError>>
    where
        Self: 'a;

    /// Lists all branches in the repository.
    /// This function returns a stream of branch ids.
    fn branches<'a>(&'a self) -> Self::ListIter<'a>;

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
    fn head(&self, id: Id) -> Result<Option<Value<Handle<H, SimpleArchive>>>, Self::HeadError>;

    /// Puts a branch on the repository, creating or updating it.
    ///
    /// # Parameters
    /// * `old` - Expected current value of the branch (None if creating new)
    /// * `new` - Value to update the branch to
    ///
    /// # Returns
    /// * `Success` - Push completed successfully
    /// * `Conflict(current)` - Failed because the branch's current value doesn't match `old`
    ///   (contains the actual current value for conflict resolution)
    fn update(
        &mut self,
        id: Id,
        old: Option<Value<Handle<H, SimpleArchive>>>,
        new: Value<Handle<H, SimpleArchive>>,
    ) -> Result<PushResult<H>, Self::UpdateError>;
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

/// Copies every blob from `source` into `target`.
///
/// The returned iterator yields a `(old, new)` handle pair for each transferred
/// blob, allowing callers to update references from the source store to the
/// newly inserted blobs in the target.
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
            <BS as BlobStoreList<HS>>::Err,
            <BS as BlobStoreGet<HS>>::GetError<Infallible>,
            <BT as BlobStorePut<HT>>::PutError,
        >,
    >,
> + 'a
where
    BS: BlobStoreList<HS> + BlobStoreGet<HS>,
    BT: BlobStorePut<HT>,
    HS: 'static + HashProtocol,
    HT: 'static + HashProtocol,
{
    source.blobs().map(
        move |source_handle: Result<
            Value<Handle<HS, UnknownBlob>>,
            <BS as BlobStoreList<HS>>::Err,
        >| {
            let source_handle = source_handle.map_err(|e| TransferError::List(e))?;
            let blob: Blob<UnknownBlob> = source
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
pub enum MergeError {
    /// The merge failed because the workspaces have different base repos.
    DifferentRepos(),
}

#[derive(Debug)]
pub enum PushError<Storage: BranchStore<Blake3> + BlobStore<Blake3>> {
    /// An error occurred while enumerating the branch storage branches.
    StorageBranches(Storage::BranchesError),
    /// An error occurred while reading metadata blobs.
    StorageGet(
        <<Storage as BlobStore<Blake3>>::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>,
    ),
    /// An error occurred while transferring blobs to the repository.
    StoragePut(<Storage as BlobStorePut<Blake3>>::PutError),
    /// An error occurred while updating the branch storage.
    BranchUpdate(Storage::UpdateError),
    /// Malformed branch metadata.
    BadBranchMetadata(),
}

#[derive(Debug)]
pub enum BranchError<Storage>
where
    Storage: BranchStore<Blake3> + BlobStore<Blake3>,
{
    /// An error occurred while reading metadata blobs.
    StorageGet(
        <<Storage as BlobStore<Blake3>>::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>,
    ),
    /// An error occurred while storing blobs.
    StoragePut(<Storage as BlobStorePut<Blake3>>::PutError),
    /// An error occurred while retrieving branch heads.
    BranchHead(Storage::HeadError),
    /// An error occurred while updating the branch storage.
    BranchUpdate(Storage::UpdateError),
    /// The branch already exists.
    AlreadyExists(),
    /// The referenced base branch does not exist.
    BranchNotFound(Id),
}

/// High-level wrapper combining a blob store and branch store into a usable
/// repository API.
///
/// The `Repository` type exposes convenience methods for creating branches,
/// committing data and pushing changes while delegating actual storage to the
/// given `BlobStore` and `BranchStore` implementations.
pub struct Repository<Storage: BlobStore<Blake3> + BranchStore<Blake3>> {
    storage: Storage,
    signing_key: SigningKey,
}

pub enum CheckoutError<BranchStorageErr, BlobStorageErr>
where
    BranchStorageErr: Error,
    BlobStorageErr: Error,
{
    /// The branch does not exist in the repository.
    BranchNotFound(Id),
    /// An error occurred while accessing the branch storage.
    BranchStorage(BranchStorageErr),
    /// An error occurred while accessing the blob storage.
    BlobStorage(BlobStorageErr),
    /// The branch metadata is malformed or does not contain the expected fields.
    BadBranchMetadata(),
}

impl<B, C> fmt::Debug for CheckoutError<B, C>
where
    B: Error + fmt::Debug,
    C: Error + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckoutError::BranchNotFound(id) => f.debug_tuple("BranchNotFound").field(id).finish(),
            CheckoutError::BranchStorage(e) => f.debug_tuple("BranchStorage").field(e).finish(),
            CheckoutError::BlobStorage(e) => f.debug_tuple("BlobStorage").field(e).finish(),
            CheckoutError::BadBranchMetadata() => f.debug_tuple("BadBranchMetadata").finish(),
        }
    }
}

impl<Storage> Repository<Storage>
where
    Storage: BlobStore<Blake3> + BranchStore<Blake3>,
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
    pub fn new(storage: Storage, signing_key: SigningKey) -> Self {
        Self {
            storage,
            signing_key,
        }
    }

    /// Replace the repository signing key.
    pub fn set_signing_key(&mut self, signing_key: SigningKey) {
        self.signing_key = signing_key;
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
    ///
    /// # Returns
    /// A new workspace bound to the created branch.
    ///
    pub fn branch(
        &mut self,
        branch_name: &str,
    ) -> Result<Workspace<Storage>, BranchError<Storage>> {
        self.branch_with_key(branch_name, self.signing_key.clone())
    }

    /// Creates a new branch with an explicit signing key.
    pub fn branch_with_key(
        &mut self,
        branch_name: &str,
        signing_key: SigningKey,
    ) -> Result<Workspace<Storage>, BranchError<Storage>> {
        let branch_id = *ufoid();
        let branch_set = branch::branch_unsigned(branch_id, branch_name, None);
        let branch_blob = branch_set.to_blob();
        let branch_handle = self
            .storage
            .put(branch_blob)
            .map_err(|e| BranchError::StoragePut(e))?;

        let push_result = self
            .storage
            .update(branch_id, None, branch_handle)
            .map_err(|e| BranchError::BranchUpdate(e))?;

        match push_result {
            PushResult::Success() => Ok(Workspace {
                base_blobs: self.storage.reader(),
                local_blobs: MemoryBlobStore::new(),
                head: None,
                base_branch_id: branch_id,
                base_branch_meta: branch_handle,
                signing_key,
            }),
            PushResult::Conflict(_) => Err(BranchError::AlreadyExists()),
        }
    }

    /// Creates a new branch starting from an existing commit.
    ///
    /// * `branch_name` - Name of the new branch.
    /// * `commit` - Commit to initialize the branch from.
    pub fn branch_from(
        &mut self,
        branch_name: &str,
        commit: CommitHandle,
    ) -> Result<Workspace<Storage>, BranchError<Storage>> {
        self.branch_from_with_key(branch_name, commit, self.signing_key.clone())
    }

    /// Same as [`branch_from`] but uses the provided signing key.
    pub fn branch_from_with_key(
        &mut self,
        branch_name: &str,
        commit: CommitHandle,
        signing_key: SigningKey,
    ) -> Result<Workspace<Storage>, BranchError<Storage>> {
        let branch_id = *ufoid();

        let set: TribleSet = self
            .storage
            .reader()
            .get(commit)
            .map_err(|e| BranchError::StorageGet(e))?;

        let branch_set = branch(&signing_key, branch_id, branch_name, Some(set.to_blob()));
        let branch_blob = branch_set.to_blob();
        let branch_handle = self
            .storage
            .put(branch_blob)
            .map_err(|e| BranchError::StoragePut(e))?;

        let push_result = self
            .storage
            .update(branch_id, None, branch_handle)
            .map_err(|e| BranchError::BranchUpdate(e))?;

        match push_result {
            PushResult::Success() => Ok(Workspace {
                base_blobs: self.storage.reader(),
                local_blobs: MemoryBlobStore::new(),
                head: Some(commit),
                base_branch_id: branch_id,
                base_branch_meta: branch_handle,
                signing_key,
            }),
            PushResult::Conflict(_) => Err(BranchError::AlreadyExists()),
        }
    }

    /// Checks out an existing branch using the repository's signing key.
    pub fn checkout(
        &mut self,
        branch_id: Id,
    ) -> Result<
        Workspace<Storage>,
        CheckoutError<
            Storage::HeadError,
            <Storage::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>,
        >,
    > {
        self.checkout_with_key(branch_id, self.signing_key.clone())
    }

    /// Same as [`checkout`] but overrides the signing key.
    pub fn checkout_with_key(
        &mut self,
        branch_id: Id,
        signing_key: SigningKey,
    ) -> Result<
        Workspace<Storage>,
        CheckoutError<
            Storage::HeadError,
            <Storage::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>,
        >,
    > {
        // 1. Get the branch metadata head from the branch store.
        let base_branch_meta_handle = match self.storage.head(branch_id) {
            Ok(Some(handle)) => handle,
            Ok(None) => return Err(CheckoutError::BranchNotFound(branch_id)),
            Err(e) => return Err(CheckoutError::BranchStorage(e)),
        };
        // 2. Get the current commit from the branch metadata.
        let base_branch_meta: TribleSet = match self.storage.reader().get(base_branch_meta_handle) {
            Ok(metadata) => metadata,
            Err(e) => return Err(CheckoutError::BlobStorage(e)),
        };

        let head = match find!(
            (head: Value<_>),
            repo::pattern!(&base_branch_meta, [{ head: head }])
        )
        .at_most_one()
        {
            Ok(Some((h,))) => Some(h),
            Ok(None) => None,
            Err(_) => return Err(CheckoutError::BadBranchMetadata()),
        };
        // Create workspace with the current commit and base blobs.
        Ok(Workspace {
            base_blobs: self.storage.reader(),
            local_blobs: MemoryBlobStore::new(),
            head,
            base_branch_id: branch_id,
            base_branch_meta: base_branch_meta_handle,
            signing_key,
        })
    }

    /// Pushes the workspace's new blobs and commit to the persistent repository.
    /// This syncs the local BlobSet with the repository's BlobStore and performs
    /// an atomic branch update (using the stored base_branch_meta).
    pub fn push(
        &mut self,
        workspace: &mut Workspace<Storage>,
    ) -> Result<Option<Workspace<Storage>>, PushError<Storage>> {
        // 1. Sync `self.local_blobset` to repository's BlobStore.
        let workspace_reader = workspace.local_blobs.reader();
        for handle in workspace_reader.blobs() {
            let handle = handle.expect("infallible blob enumeration");
            let blob: Blob<UnknownBlob> =
                workspace_reader.get(handle).expect("infallible blob read");
            self.storage
                .put(blob)
                .map_err(|e| PushError::StoragePut(e))?;
        }
        // 2. Create a new branch meta blob referencing the new workspace head.
        let repo_reader = self.storage.reader();

        let base_branch_meta: TribleSet = repo_reader
            .get(workspace.base_branch_meta)
            .map_err(|e| PushError::StorageGet(e))?;

        let Ok((branch_name,)) = find!((name: Value<_>),
            metadata::pattern!(base_branch_meta, [{ name: name }])
        )
        .exactly_one() else {
            return Err(PushError::BadBranchMetadata());
        };

        let head_handle = workspace.head.ok_or(PushError::BadBranchMetadata())?;
        let head: TribleSet = repo_reader
            .get(head_handle)
            .map_err(|e| PushError::StorageGet(e))?;

        let branch_meta = branch(
            &workspace.signing_key,
            workspace.base_branch_id,
            branch_name.from_value(),
            Some(head.to_blob()),
        );

        let branch_meta_handle = self
            .storage
            .put(branch_meta)
            .map_err(|e| PushError::StoragePut(e))?;

        // 3. Use CAS (comparing against self.base_branch_meta) to update the branch pointer.

        let result = self
            .storage
            .update(
                workspace.base_branch_id,
                Some(workspace.base_branch_meta),
                branch_meta_handle,
            )
            .map_err(|e| PushError::BranchUpdate(e))?;

        match result {
            PushResult::Success() => Ok(None),
            PushResult::Conflict(conflicting_meta) => {
                let conflicting_meta = conflicting_meta.ok_or(PushError::BadBranchMetadata())?;

                let repo_reader = self.storage.reader();
                let branch_meta: TribleSet = repo_reader
                    .get(conflicting_meta)
                    .map_err(|e| PushError::StorageGet(e))?;

                let head = match find!((head: Value<_>),
                    repo::pattern!(&branch_meta, [{ head: head }])
                )
                .at_most_one()
                {
                    Ok(Some((h,))) => Some(h),
                    Ok(None) => None,
                    Err(_) => return Err(PushError::BadBranchMetadata()),
                };

                let conflict_ws = Workspace {
                    base_blobs: self.storage.reader(),
                    local_blobs: MemoryBlobStore::new(),
                    head,
                    base_branch_id: workspace.base_branch_id,
                    base_branch_meta: conflicting_meta,
                    signing_key: workspace.signing_key.clone(),
                };

                Ok(Some(conflict_ws))
            }
        }
    }
}

type CommitHandle = Value<Handle<Blake3, SimpleArchive>>;
type BranchMetaHandle = Value<Handle<Blake3, SimpleArchive>>;

/// The Workspace represents the mutable working area or "staging" state.
/// It was formerly known as `Head`. It is sent to worker threads,
/// modified (via commits, merges, etc.), and then merged back into the Repository.
pub struct Workspace<Blobs: BlobStore<Blake3>> {
    /// A local BlobStore that holds any new blobs (commits, trees, deltas) before they are synced.
    local_blobs: MemoryBlobStore<Blake3>,
    /// The blob storage base for the workspace.
    base_blobs: Blobs::Reader,
    /// The branch id this workspace is tracking; None for a detached workspace.
    base_branch_id: Id,
    /// The meta-handle corresponding to the base branch state used for CAS.
    base_branch_meta: BranchMetaHandle,
    /// Handle to the current commit in the working branch. `None` for an empty branch.
    head: Option<CommitHandle>,
    /// Signing key used for commit/branch signing.
    signing_key: SigningKey,
}

impl<Blobs> fmt::Debug for Workspace<Blobs>
where
    Blobs: BlobStore<Blake3>,
    Blobs::Reader: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Workspace")
            .field("local_blobs", &self.local_blobs)
            .field("base_blobs", &self.base_blobs)
            .field("base_branch_id", &self.base_branch_id)
            .field("base_branch_meta", &self.base_branch_meta)
            .field("head", &self.head)
            .finish()
    }
}

impl<Blobs: BlobStore<Blake3>> Workspace<Blobs> {
    /// Returns the branch id associated with this workspace.
    pub fn branch_id(&self) -> Id {
        self.base_branch_id
    }

    /// Adds a blob to the workspace's local blob store.
    /// Mirrors [`BlobStorePut::put`](crate::repo::BlobStorePut) for ease of use.
    pub fn put<S, T>(&mut self, item: T) -> Value<Handle<Blake3, S>>
    where
        S: BlobSchema + 'static,
        T: ToBlob<S>,
        Handle<Blake3, S>: ValueSchema,
    {
        self.local_blobs.put(item).expect("infallible blob put")
    }

    /// Performs a commit in the workspace.
    /// This method creates a new commit blob (stored in the local blobset)
    /// and updates the current commit handle.
    pub fn commit(&mut self, content: TribleSet, message: Option<&str>) {
        // 1. Create a commit blob from the current head, content and the commit message (if any).
        let content_blob = content.to_blob();
        let parents = self.head.iter().copied();
        let commit_set = crate::repo::commit::commit(
            &self.signing_key,
            parents,
            message,
            Some(content_blob.clone()),
        );
        // 2. Store the content and commit blobs in `self.local_blobs`.
        let _ = self
            .local_blobs
            .put(content_blob)
            .expect("failed to put content blob");
        let commit_handle = self
            .local_blobs
            .put(commit_set)
            .expect("failed to put commit blob");
        // 3. Update `self.head` to point to the new commit.
        self.head = Some(commit_handle);
    }

    /// Merges another workspace (or its commit state) into this one.
    /// This returns a new commit that combines the changes from both.
    pub fn merge(&mut self, other: &mut Workspace<Blobs>) -> Result<CommitHandle, MergeError> {
        if self.base_blobs != other.base_blobs {
            // Cannot merge workspaces with different base blobs,
            // as this would potentially require transferring a huge number of blobs
            // between the merged workspace to the current one.
            // A better design would be to transfer the blobs first,
            // then merge the commit states via detached commits.
            return Err(MergeError::DifferentRepos());
        }
        // 1. Transfer all blobs from the other workspace to self.local_blobs.
        let other_local = other.local_blobs.reader();
        for r in other_local.blobs() {
            let handle = r.expect("infallible blob enumeration");
            let blob: Blob<UnknownBlob> = other_local.get(handle).expect("infallible blob read");

            // Store the blob in the local workspace's blob store.
            self.local_blobs.put(blob).expect("infallible blob put");
        }
        // 2. Compute a merge commit from self.current_commit and other.current_commit.
        let parents = self.head.iter().copied().chain(other.head.iter().copied());
        let merge_commit = commit(
            &self.signing_key,
            parents,
            None, // No message for the merge commit
            None, // No content blob for the merge commit
        );
        // 3. Store the merge commit in self.local_blobs.
        let commit_handle = self
            .local_blobs
            .put(merge_commit)
            .expect("failed to put merge commit blob");
        self.head = Some(commit_handle);

        Ok(commit_handle)
    }
}
