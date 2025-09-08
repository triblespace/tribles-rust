//! This module provides a high-level API for storing and retrieving data from repositories.
//! The design is inspired by Git, but with a focus on object/content-addressed storage.
//! It separates storage concerns from the data model, and reduces the mutable state of the repository,
//! to an absolute minimum, making it easier to reason about and allowing for different storage backends.
//!
//! Blob repositories are collections of blobs that can be content-addressed by their hash.
#![allow(clippy::type_complexity)]
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
//! fields! {
//!     "8F180883F9FD5F787E9E0AF0DF5866B9" as pub author: GenId;
//!     "0DBB530B37B966D137C50B943700EDB2" as pub firstname: ShortString;
//!     "6BAA463FD4EAF45F6A103DB9433E4545" as pub lastname: ShortString;
//! }
//! let author = fucid();
//! ws.commit(
//!     entity!(&author, {
//!         literature::firstname: "Frank",
//!         literature::lastname: "Herbert",
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
//! `pull` creates a new workspace from an existing branch while
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
//! - `pull` is similar to cloning a branch into a new workspace.
//!
//! `pull` uses the repository's default signing key for new commits. If you
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

/// Trait for storage backends that require explicit close/cleanup.
///
/// Not all storage backends need to implement this; implementations that have
/// nothing to do on close may return Ok(()) or use `Infallible` as the error
/// type.
pub trait StorageClose {
    /// Error type returned by `close`.
    type Error: std::error::Error;

    /// Consume the storage and perform any necessary cleanup.
    fn close(self) -> Result<(), Self::Error>;
}

// Convenience impl for repositories whose storage supports explicit close.
impl<Storage> Repository<Storage>
where
    Storage: BlobStore<Blake3> + BranchStore<Blake3> + StorageClose,
{
    /// Close the repository's underlying storage if it supports explicit
    /// close operations.
    ///
    /// This method is only available when the storage type implements
    /// [`StorageClose`]. It consumes the repository and delegates to the
    /// storage's `close` implementation, returning any error produced.
    pub fn close(self) -> Result<(), <Storage as StorageClose>::Error> {
        self.storage.close()
    }
}

use std::collections::HashSet;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::{self};
use crate::pattern;


use commit::commit_metadata;
use hifitime::Epoch;
use itertools::Itertools;

use crate::blob::schemas::simplearchive::UnarchiveError;
use crate::blob::schemas::UnknownBlob;
use crate::blob::Blob;
use crate::blob::BlobSchema;
use crate::blob::MemoryBlobStore;
use crate::blob::ToBlob;
use crate::blob::TryFromBlob;
use crate::find;
use crate::id::ufoid;
use crate::id::Id;
use crate::metadata;
use crate::patch::Entry;
use crate::patch::IdentitySchema;
use crate::patch::PATCH;
use crate::prelude::valueschemas::GenId;
use crate::repo::branch::branch_metadata;
use crate::trible::TribleSet;
use crate::value::schemas::hash::Handle;
use crate::value::schemas::hash::HashProtocol;
use crate::value::Value;
use crate::value::ValueSchema;
use crate::value::VALUE_LEN;
use ed25519_dalek::SigningKey;

use crate::blob::schemas::longstring::LongString;
use crate::blob::schemas::simplearchive::SimpleArchive;
use crate::value::schemas::hash::Blake3;
use crate::prelude::*;
use crate::value::schemas::shortstring::ShortString;
use crate::value::schemas::time::NsTAIInterval;
use crate::value::schemas::ed25519 as ed;

fields!{
    /// The actual data of the commit.
    "4DD4DDD05CC31734B03ABB4E43188B1F" as pub content: Handle<Blake3, SimpleArchive>;
    /// A commit that this commit is based on.
    "317044B612C690000D798CA660ECFD2A" as pub parent: Handle<Blake3, SimpleArchive>;
    /// A (potentially long) message describing the commit.
    "B59D147839100B6ED4B165DF76EDF3BB" as pub message: Handle<Blake3, LongString>;
    /// A short message describing the commit.
    "12290C0BE0E9207E324F24DDE0D89300" as pub short_message: ShortString;
    /// The hash of the first commit in the commit chain of the branch.
    "272FBC56108F336C4D2E17289468C35F" as pub head: Handle<Blake3, SimpleArchive>;
    /// An id used to track the branch.
    "8694CC73AF96A5E1C7635C677D1B928A" as pub branch: GenId;
    /// Timestamp range when this commit was created.
    "71FF566AB4E3119FC2C5E66A18979586" as pub timestamp: NsTAIInterval;
    /// The author of the signature identified by their ed25519 public key.
    "ADB4FFAD247C886848161297EFF5A05B" as pub signed_by: ed::ED25519PublicKey;
    /// The `r` part of a ed25519 signature.
    "9DF34F84959928F93A3C40AEB6E9E499" as pub signature_r: ed::ED25519RComponent;
    /// The `s` part of a ed25519 signature.
    "1ACE03BF70242B289FDF00E4327C3BC6" as pub signature_s: ed::ED25519SComponent;
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
    type ReaderError: Error + Debug + Send + Sync + 'static;
    fn reader(&mut self) -> Result<Self::Reader, Self::ReaderError>;
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
    fn branches<'a>(&'a mut self) -> Result<Self::ListIter<'a>, Self::BranchesError>;

    // NOTE: keep the API lean — callers may call `branches()` and handle the
    // fallible iterator directly; we avoid adding an extra helper here.

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
    fn head(&mut self, id: Id) -> Result<Option<Value<Handle<H, SimpleArchive>>>, Self::HeadError>;

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
            let source_handle = source_handle.map_err(TransferError::List)?;
            let blob: Blob<UnknownBlob> = source.get(source_handle).map_err(TransferError::Load)?;
            let target_handle = target.put(blob).map_err(TransferError::Store)?;
            Ok((source_handle, target_handle))
        },
    )
}

/// Statistics returned by `copy_reachable`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CopyReachableStats {
    /// Number of distinct blob handles discovered (visited).
    pub visited: usize,
    /// Number of blobs written into the target store (including de‑duped writes).
    pub stored: usize,
}

/// Error type for `copy_reachable` operations.
#[derive(Debug)]
pub enum CopyReachableError<LoadErr, StoreErr>
where
    LoadErr: Error + Debug + 'static,
    StoreErr: Error + Debug + 'static,
{
    Load(LoadErr),
    Store(StoreErr),
}

impl<LoadErr, StoreErr> fmt::Display for CopyReachableError<LoadErr, StoreErr>
where
    LoadErr: Error + Debug + 'static,
    StoreErr: Error + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(e) => write!(f, "load error: {e}"),
            Self::Store(e) => write!(f, "store error: {e}"),
        }
    }
}

impl<LoadErr, StoreErr> Error for CopyReachableError<LoadErr, StoreErr>
where
    LoadErr: Error + Debug + 'static,
    StoreErr: Error + Debug + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Load(e) => Some(e),
            Self::Store(e) => Some(e),
        }
    }
}

/// Conservatively copy all blobs reachable from the provided root handles in `source`
/// into `target` by scanning each loaded blob's bytes for 32‑byte aligned chunks and
/// attempting to treat each as a hash handle in the same hash protocol.
///
/// - Requires no schema/namespace knowledge and is robust to future additions.
/// - Duplicate blobs are naturally de‑duplicated by the target store's hashing.
/// - The traversal starts from the provided roots (e.g., a branch head commit handle).
pub fn copy_reachable<BS, BT, H>(
    source: &BS,
    target: &mut BT,
    roots: impl IntoIterator<Item = Value<Handle<H, UnknownBlob>>>,
) -> Result<
    CopyReachableStats,
    CopyReachableError<
        <BS as BlobStoreGet<H>>::GetError<Infallible>,
        <BT as BlobStorePut<H>>::PutError,
    >,
>
where
    BS: BlobStoreGet<H>,
    BT: BlobStorePut<H>,
    H: 'static + HashProtocol,
{
    use std::collections::HashSet;
    use std::collections::VecDeque;

    let mut visited: HashSet<[u8; 32]> = HashSet::new();
    let mut queue: VecDeque<Value<Handle<H, UnknownBlob>>> = VecDeque::new();
    for r in roots.into_iter() {
        queue.push_back(r);
    }

    let mut stats = CopyReachableStats::default();

    while let Some(handle) = queue.pop_front() {
        let raw: [u8; 32] = handle.raw;
        if !visited.insert(raw) {
            continue;
        }
        stats.visited += 1;

        // Load blob from source; skip if missing.
        let blob: Blob<UnknownBlob> = match source.get(handle) {
            Ok(b) => b,
            Err(_e) => {
                // Not present in source; ignore.
                continue;
            }
        };

        // Store into target (de‑dup handled by storage layer).
        let _ = target
            .put(blob.clone())
            .map_err(CopyReachableError::Store)?;
        stats.stored += 1;

        // Scan bytes for 32‑byte aligned candidates; push if load succeeds.
        let bytes: &[u8] = blob.bytes.as_ref();
        let mut i = 0usize;
        while i + 32 <= bytes.len() {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes[i..i + 32]);
            let cand: Value<Handle<H, UnknownBlob>> = Value::new(arr);
            if !visited.contains(&arr) && source.get::<anybytes::Bytes, UnknownBlob>(cand).is_ok() {
                queue.push_back(cand);
            }
            i += 32;
        }
    }

    Ok(stats)
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
            CreateCommitError::ContentStorageError(e) => write!(f, "Content storage failed: {e}"),
            CreateCommitError::CommitStorageError(e) => {
                write!(f, "Commit metadata storage failed: {e}")
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
    /// An error occurred while creating a blob reader.
    StorageReader(<Storage as BlobStore<Blake3>>::ReaderError),
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
    /// An error occurred while creating a blob reader.
    StorageReader(<Storage as BlobStore<Blake3>>::ReaderError),
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

#[derive(Debug)]
pub enum LookupError<Storage>
where
    Storage: BranchStore<Blake3> + BlobStore<Blake3>,
{
    StorageBranches(Storage::BranchesError),
    BranchHead(Storage::HeadError),
    StorageReader(<Storage as BlobStore<Blake3>>::ReaderError),
    StorageGet(
        <<Storage as BlobStore<Blake3>>::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>,
    ),
    /// Multiple branches were found with the given name.
    NameConflict(Vec<Id>),
    BadBranchMetadata(),
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

pub enum PullError<BranchStorageErr, BlobReaderErr, BlobStorageErr>
where
    BranchStorageErr: Error,
    BlobReaderErr: Error,
    BlobStorageErr: Error,
{
    /// The branch does not exist in the repository.
    BranchNotFound(Id),
    /// An error occurred while accessing the branch storage.
    BranchStorage(BranchStorageErr),
    /// An error occurred while creating a blob reader.
    BlobReader(BlobReaderErr),
    /// An error occurred while accessing the blob storage.
    BlobStorage(BlobStorageErr),
    /// The branch metadata is malformed or does not contain the expected fields.
    BadBranchMetadata(),
}

impl<B, R, C> fmt::Debug for PullError<B, R, C>
where
    B: Error + fmt::Debug,
    R: Error + fmt::Debug,
    C: Error + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PullError::BranchNotFound(id) => f.debug_tuple("BranchNotFound").field(id).finish(),
            PullError::BranchStorage(e) => f.debug_tuple("BranchStorage").field(e).finish(),
            PullError::BlobReader(e) => f.debug_tuple("BlobReader").field(e).finish(),
            PullError::BlobStorage(e) => f.debug_tuple("BlobStorage").field(e).finish(),
            PullError::BadBranchMetadata() => f.debug_tuple("BadBranchMetadata").finish(),
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

    /// Consume the repository and return the underlying storage backend.
    ///
    /// This is useful for callers that need to take ownership of the storage
    /// (for example to call `close()` on a `Pile`) instead of letting the
    /// repository drop it implicitly.
    pub fn into_storage(self) -> Storage {
        self.storage
    }

    /// Replace the repository signing key.
    pub fn set_signing_key(&mut self, signing_key: SigningKey) {
        self.signing_key = signing_key;
    }

    /// Initializes a new branch in the repository.
    /// Branches are the only mutable state in the repository,
    /// and are used to represent the state of a commit chain at a specific point in time.
    /// A branch must always point to a commit, and this function can be used to create a new branch.
    ///
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
            PushResult::Success() => {
                let base_blobs = self
                    .storage
                    .reader()
                    .map_err(|e| BranchError::StorageReader(e))?;
                Ok(Workspace {
                    base_blobs,
                    local_blobs: MemoryBlobStore::new(),
                    head: None,
                    base_branch_id: branch_id,
                    base_branch_meta: branch_handle,
                    signing_key,
                })
            }
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

        let reader = self
            .storage
            .reader()
            .map_err(|e| BranchError::StorageReader(e))?;
        let set: TribleSet = reader.get(commit).map_err(|e| BranchError::StorageGet(e))?;

        let branch_set = branch_metadata(&signing_key, branch_id, branch_name, Some(set.to_blob()));
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
            PushResult::Success() => {
                let base_blobs = self
                    .storage
                    .reader()
                    .map_err(|e| BranchError::StorageReader(e))?;
                Ok(Workspace {
                    base_blobs,
                    local_blobs: MemoryBlobStore::new(),
                    head: Some(commit),
                    base_branch_id: branch_id,
                    base_branch_meta: branch_handle,
                    signing_key,
                })
            }
            PushResult::Conflict(_) => Err(BranchError::AlreadyExists()),
        }
    }

    /// Pulls an existing branch using the repository's signing key.
    pub fn pull(
        &mut self,
        branch_id: Id,
    ) -> Result<
        Workspace<Storage>,
        PullError<
            Storage::HeadError,
            Storage::ReaderError,
            <Storage::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>,
        >,
    > {
        self.pull_with_key(branch_id, self.signing_key.clone())
    }

    /// Same as [`pull`] but overrides the signing key.
    pub fn pull_with_key(
        &mut self,
        branch_id: Id,
        signing_key: SigningKey,
    ) -> Result<
        Workspace<Storage>,
        PullError<
            Storage::HeadError,
            Storage::ReaderError,
            <Storage::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>,
        >,
    > {
        // 1. Get the branch metadata head from the branch store.
        let base_branch_meta_handle = match self.storage.head(branch_id) {
            Ok(Some(handle)) => handle,
            Ok(None) => return Err(PullError::BranchNotFound(branch_id)),
            Err(e) => return Err(PullError::BranchStorage(e)),
        };
        // 2. Get the current commit from the branch metadata.
        let reader = self.storage.reader().map_err(PullError::BlobReader)?;
        let base_branch_meta: TribleSet = match reader.get(base_branch_meta_handle) {
            Ok(metadata) => metadata,
            Err(e) => return Err(PullError::BlobStorage(e)),
        };

        let head_ = match find!(
            (head_: Value<_>),
            pattern!(&base_branch_meta, [{ head: head_ }])
        )
        .at_most_one()
        {
            Ok(Some((h,))) => Some(h),
            Ok(None) => None,
            Err(_) => return Err(PullError::BadBranchMetadata()),
        };
        // Create workspace with the current commit and base blobs.
        let base_blobs = self.storage.reader().map_err(PullError::BlobReader)?;
        Ok(Workspace {
            base_blobs,
            local_blobs: MemoryBlobStore::new(),
            head: head_,
            base_branch_id: branch_id,
            base_branch_meta: base_branch_meta_handle,
            signing_key,
        })
    }

    /// Find the id of a branch by its name.
    pub fn branch_id_by_name(&mut self, name: &str) -> Result<Option<Id>, LookupError<Storage>> {
        let ids: Vec<Id> = {
            let iter = self
                .storage
                .branches()
                .map_err(LookupError::StorageBranches)?;
            iter.map(|r| r.map_err(|e| LookupError::StorageBranches(e)))
                .collect::<Result<_, _>>()?
        };

        let mut handles = Vec::new();
        for id in ids {
            if let Some(handle) = self
                .storage
                .head(id)
                .map_err(|e| LookupError::BranchHead(e))?
            {
                handles.push((id, handle));
            }
        }

        let reader = self
            .storage
            .reader()
            .map_err(|e| LookupError::StorageReader(e))?;
        let mut matches = Vec::new();
        for (id, handle) in handles {
            let meta: TribleSet = reader.get(handle).map_err(|e| LookupError::StorageGet(e))?;

            let branch_name = find!((n: Value<_>), pattern!(meta, [{ metadata::name: n }]))
                .exactly_one()
                .map_err(|_| LookupError::BadBranchMetadata())?
                .0;

            if branch_name.from_value::<String>() == name {
                matches.push(id);
            }
        }

        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches[0])),
            _ => Err(LookupError::NameConflict(matches)),
        }
    }

    /// Pushes the workspace's new blobs and commit to the persistent repository.
    /// This syncs the local BlobSet with the repository's BlobStore and performs
    /// an atomic branch update (using the stored base_branch_meta).
    pub fn push(
        &mut self,
        workspace: &mut Workspace<Storage>,
    ) -> Result<Option<Workspace<Storage>>, PushError<Storage>> {
        // 1. Sync `self.local_blobset` to repository's BlobStore.
        let workspace_reader = workspace.local_blobs.reader().unwrap();
        for handle in workspace_reader.blobs() {
            let handle = handle.expect("infallible blob enumeration");
            let blob: Blob<UnknownBlob> =
                workspace_reader.get(handle).expect("infallible blob read");
            self.storage
                .put(blob)
                .map_err(|e| PushError::StoragePut(e))?;
        }
        // 2. Create a new branch meta blob referencing the new workspace head.
        let repo_reader = self
            .storage
            .reader()
            .map_err(|e| PushError::StorageReader(e))?;

        let base_branch_meta: TribleSet = repo_reader
            .get(workspace.base_branch_meta)
            .map_err(|e| PushError::StorageGet(e))?;

        let Ok((branch_name,)) = find!((name: Value<_>),
            pattern!(base_branch_meta, [{ metadata::name: name }])
        )
        .exactly_one() else {
            return Err(PushError::BadBranchMetadata());
        };

        let head_handle = workspace.head.ok_or(PushError::BadBranchMetadata())?;
        let head_: TribleSet = repo_reader
            .get(head_handle)
            .map_err(|e| PushError::StorageGet(e))?;

        let branch_meta = branch_metadata(
            &workspace.signing_key,
            workspace.base_branch_id,
            branch_name.from_value(),
            Some(head_.to_blob()),
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

                let repo_reader = self
                    .storage
                    .reader()
                    .map_err(|e| PushError::StorageReader(e))?;
                let branch_meta: TribleSet = repo_reader
                    .get(conflicting_meta)
                    .map_err(|e| PushError::StorageGet(e))?;

                let head_ = match find!((head_: Value<_>),
                    pattern!(&branch_meta, [{ head: head_ }])
                )
                .at_most_one()
                {
                    Ok(Some((h,))) => Some(h),
                    Ok(None) => None,
                    Err(_) => return Err(PushError::BadBranchMetadata()),
                };

                let conflict_ws = Workspace {
                    base_blobs: self
                        .storage
                        .reader()
                        .map_err(|e| PushError::StorageReader(e))?,
                    local_blobs: MemoryBlobStore::new(),
                    head: head_,
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
type CommitSet = PATCH<VALUE_LEN, IdentitySchema, ()>;
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

/// Helper trait for [`Workspace::checkout`] specifying commit handles or ranges.
pub trait CommitSelector<Blobs: BlobStore<Blake3>> {
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    >;
}

/// Selector that returns a commit along with all of its ancestors.
pub struct Ancestors(pub CommitHandle);

/// Convenience function to create an [`Ancestors`] selector.
pub fn ancestors(commit: CommitHandle) -> Ancestors {
    Ancestors(commit)
}

/// Selector that returns the Nth ancestor along the first-parent chain.
pub struct NthAncestor(pub CommitHandle, pub usize);

/// Convenience function to create an [`NthAncestor`] selector.
pub fn nth_ancestor(commit: CommitHandle, n: usize) -> NthAncestor {
    NthAncestor(commit, n)
}

/// Selector that returns the direct parents of a commit.
pub struct Parents(pub CommitHandle);

/// Convenience function to create a [`Parents`] selector.
pub fn parents(commit: CommitHandle) -> Parents {
    Parents(commit)
}

/// Selector that returns commits reachable from either of two commits but not
/// both.
pub struct SymmetricDiff(pub CommitHandle, pub CommitHandle);

/// Convenience function to create a [`SymmetricDiff`] selector.
pub fn symmetric_diff(a: CommitHandle, b: CommitHandle) -> SymmetricDiff {
    SymmetricDiff(a, b)
}

/// Selector that returns commits with timestamps in the given inclusive range.
pub struct TimeRange(pub Epoch, pub Epoch);

/// Convenience function to create a [`TimeRange`] selector.
pub fn time_range(start: Epoch, end: Epoch) -> TimeRange {
    TimeRange(start, end)
}

/// Selector that filters commits returned by another selector.
pub struct Filter<S, F> {
    selector: S,
    filter: F,
}

/// Convenience function to create a [`Filter`] selector.
pub fn filter<S, F>(selector: S, filter: F) -> Filter<S, F> {
    Filter { selector, filter }
}

impl<Blobs> CommitSelector<Blobs> for CommitHandle
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        _ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let mut patch = CommitSet::new();
        patch.insert(&Entry::new(&self.raw));
        Ok(patch)
    }
}

impl<Blobs> CommitSelector<Blobs> for Vec<CommitHandle>
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        _ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let mut patch = CommitSet::new();
        for handle in self {
            patch.insert(&Entry::new(&handle.raw));
        }
        Ok(patch)
    }
}

impl<Blobs> CommitSelector<Blobs> for &[CommitHandle]
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        _ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let mut patch = CommitSet::new();
        for handle in self {
            patch.insert(&Entry::new(&handle.raw));
        }
        Ok(patch)
    }
}

impl<Blobs> CommitSelector<Blobs> for Ancestors
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        collect_reachable(ws, self.0)
    }
}

impl<Blobs> CommitSelector<Blobs> for NthAncestor
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let mut current = self.0;
        let mut remaining = self.1;

        while remaining > 0 {
            let meta: TribleSet = ws.get(current).map_err(WorkspaceCheckoutError::Storage)?;
            let mut parents = find!((p: Value<_>), pattern!(&meta, [{ parent: p }]));
            let Some((p,)) = parents.next() else {
                return Ok(CommitSet::new());
            };
            current = p;
            remaining -= 1;
        }

        let mut patch = CommitSet::new();
        patch.insert(&Entry::new(&current.raw));
        Ok(patch)
    }
}

impl<Blobs> CommitSelector<Blobs> for Parents
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let meta: TribleSet = ws.get(self.0).map_err(WorkspaceCheckoutError::Storage)?;
        let mut result = CommitSet::new();
        for (p,) in find!((p: Value<_>), pattern!(&meta, [{ parent: p }])) {
            result.insert(&Entry::new(&p.raw));
        }
        Ok(result)
    }
}

impl<Blobs> CommitSelector<Blobs> for SymmetricDiff
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let a = collect_reachable(ws, self.0)?;
        let b = collect_reachable(ws, self.1)?;
        let inter = a.intersect(&b);
        let mut union = a;
        union.union(b);
        Ok(union.difference(&inter))
    }
}

impl<S, F, Blobs> CommitSelector<Blobs> for Filter<S, F>
where
    Blobs: BlobStore<Blake3>,
    S: CommitSelector<Blobs>,
    F: for<'x, 'y> Fn(&'x TribleSet, &'y TribleSet) -> bool,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let patch = self.selector.select(ws)?;
        let mut result = CommitSet::new();
        let filter = self.filter;
        for raw in patch.iter() {
            let handle = Value::new(*raw);
            let meta: TribleSet = ws.get(handle).map_err(WorkspaceCheckoutError::Storage)?;

            let Ok((content_handle,)) = find!(
                (c: Value<_>),
                pattern!(&meta, [{ content: c }])
            )
            .exactly_one() else {
                return Err(WorkspaceCheckoutError::BadCommitMetadata());
            };

            let payload: TribleSet = ws
                .get(content_handle)
                .map_err(WorkspaceCheckoutError::Storage)?;

            if filter(&meta, &payload) {
                result.insert(&Entry::new(raw));
            }
        }
        Ok(result)
    }
}

/// Selector that yields commits touching a specific entity.
pub struct HistoryOf(pub Id);

/// Convenience function to create a [`HistoryOf`] selector.
pub fn history_of(entity: Id) -> HistoryOf {
    HistoryOf(entity)
}

impl<Blobs> CommitSelector<Blobs> for HistoryOf
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let head_ = ws.head.ok_or(WorkspaceCheckoutError::NoHead)?;
        let entity = self.0;
        filter(
            ancestors(head_),
            move |_: &TribleSet, payload: &TribleSet| payload.iter().any(|t| t.e() == &entity),
        )
        .select(ws)
    }
}

impl<Blobs> CommitSelector<Blobs> for std::ops::Range<CommitHandle>
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let patch = collect_reachable(ws, self.end)?;
        let exclude = collect_reachable(ws, self.start)?;
        Ok(patch.difference(&exclude))
    }
}

impl<Blobs> CommitSelector<Blobs> for std::ops::RangeFrom<CommitHandle>
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let head_ = ws.head.ok_or(WorkspaceCheckoutError::NoHead)?;
        let patch = collect_reachable(ws, head_)?;
        let exclude = collect_reachable(ws, self.start)?;
        Ok(patch.difference(&exclude))
    }
}

impl<Blobs> CommitSelector<Blobs> for std::ops::RangeTo<CommitHandle>
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        collect_reachable(ws, self.end)
    }
}

impl<Blobs> CommitSelector<Blobs> for std::ops::RangeFull
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let head_ = ws.head.ok_or(WorkspaceCheckoutError::NoHead)?;
        collect_reachable(ws, head_)
    }
}

impl<Blobs> CommitSelector<Blobs> for TimeRange
where
    Blobs: BlobStore<Blake3>,
{
    fn select(
        self,
        ws: &mut Workspace<Blobs>,
    ) -> Result<
        CommitSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    > {
        let head_ = ws.head.ok_or(WorkspaceCheckoutError::NoHead)?;
        let start = self.0;
        let end = self.1;
        filter(
            ancestors(head_),
            move |meta: &TribleSet, _payload: &TribleSet| {
                if let Ok(Some((ts,))) =
                    find!((t: Value<_>), pattern!(meta, [{ timestamp: t }])).at_most_one()
                {
                    let (ts_start, ts_end): (Epoch, Epoch) =
                        crate::value::FromValue::from_value(&ts);
                    ts_start <= end && ts_end >= start
                } else {
                    false
                }
            },
        )
        .select(ws)
    }
}

impl<Blobs: BlobStore<Blake3>> Workspace<Blobs> {
    /// Returns the branch id associated with this workspace.
    pub fn branch_id(&self) -> Id {
        self.base_branch_id
    }

    /// Returns the current commit handle if one exists.
    pub fn head(&self) -> Option<CommitHandle> {
        self.head
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

    /// Retrieves a blob from the workspace.
    ///
    /// The method first checks the workspace's local blob store and falls back
    /// to the base blob store if the blob is not found locally.
    pub fn get<T, S>(
        &mut self,
        handle: Value<Handle<Blake3, S>>,
    ) -> Result<T, <Blobs::Reader as BlobStoreGet<Blake3>>::GetError<<T as TryFromBlob<S>>::Error>>
    where
        S: BlobSchema + 'static,
        T: TryFromBlob<S>,
        Handle<Blake3, S>: ValueSchema,
    {
        self.local_blobs
            .reader()
            .unwrap()
            .get(handle)
            .or_else(|_| self.base_blobs.get(handle))
    }

    /// Performs a commit in the workspace.
    /// This method creates a new commit blob (stored in the local blobset)
    /// and updates the current commit handle.
    pub fn commit(&mut self, content_: TribleSet, message_: Option<&str>) {
        // 1. Create a commit blob from the current head, content and the commit message (if any).
        let content_blob = content_.to_blob();
        // If a message is provided, store it as a LongString blob and pass the handle.
        let message_handle = message_.map(|m| self.put::<LongString, String>(m.to_string()));
        let parents = self.head.iter().copied();

        let commit_set = crate::repo::commit::commit_metadata(
            &self.signing_key,
            parents,
            message_handle,
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
        let other_local = other.local_blobs.reader().unwrap();
        for r in other_local.blobs() {
            let handle = r.expect("infallible blob enumeration");
            let blob: Blob<UnknownBlob> = other_local.get(handle).expect("infallible blob read");

            // Store the blob in the local workspace's blob store.
            self.local_blobs.put(blob).expect("infallible blob put");
        }
        // 2. Compute a merge commit from self.current_commit and other.current_commit.
        let parents = self.head.iter().copied().chain(other.head.iter().copied());
        let merge_commit = commit_metadata(
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

    /// Create a merge commit that ties this workspace's current head and an
    /// arbitrary other commit (already present in the underlying blob store)
    /// together without requiring another `Workspace` instance.
    ///
    /// This does not attach any content to the merge commit.
    pub fn merge_commit(
        &mut self,
        other: Value<Handle<Blake3, SimpleArchive>>,
    ) -> Result<CommitHandle, MergeError> {
        // Validate that `other` can be loaded from either local or base blobs.
        // If it cannot be loaded we still proceed with the merge; dereference
        // failures will surface later when reading history. Callers should
        // ensure `copy_reachable` was used beforehand when importing.

        let parents = self.head.iter().copied().chain(Some(other));
        let merge_commit = commit_metadata(&self.signing_key, parents, None, None);
        let commit_handle = self
            .local_blobs
            .put(merge_commit)
            .expect("failed to put merge commit blob");
        self.head = Some(commit_handle);
        Ok(commit_handle)
    }

    /// Returns the combined [`TribleSet`] for the specified commits.
    ///
    /// Each commit handle must reference a commit blob stored either in the
    /// workspace's local blob store or the repository's base store. The
    /// associated content blobs are loaded and unioned together. An error is
    /// returned if any commit or content blob is missing or malformed.
    fn checkout_commits<I>(
        &mut self,
        commits: I,
    ) -> Result<
        TribleSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    >
    where
        I: IntoIterator<Item = CommitHandle>,
    {
        let local = self.local_blobs.reader().unwrap();
        let mut result = TribleSet::new();
        for commit in commits {
            let meta: TribleSet = local
                .get(commit)
                .or_else(|_| self.base_blobs.get(commit))
                .map_err(WorkspaceCheckoutError::Storage)?;

            let Ok((c,)) = find!(
                (c: Value<_>),
                pattern!(&meta, [{ content: c }])
            )
            .exactly_one() else {
                return Err(WorkspaceCheckoutError::BadCommitMetadata());
            };

            let set: TribleSet = local
                .get(c)
                .or_else(|_| self.base_blobs.get(c))
                .map_err(WorkspaceCheckoutError::Storage)?;

            result.union(set);
        }
        Ok(result)
    }

    /// Returns the combined [`TribleSet`] for the specified commits or commit
    /// ranges. `spec` can be a single [`CommitHandle`], an iterator of handles
    /// or any of the standard range types over `CommitHandle`.
    pub fn checkout<R>(
        &mut self,
        spec: R,
    ) -> Result<
        TribleSet,
        WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
    >
    where
        R: CommitSelector<Blobs>,
    {
        let patch = spec.select(self)?;
        let commits = patch.iter().map(|raw| Value::new(*raw));
        self.checkout_commits(commits)
    }
}

#[derive(Debug)]
pub enum WorkspaceCheckoutError<GetErr: Error> {
    /// Error retrieving blobs from storage.
    Storage(GetErr),
    /// Commit metadata is malformed or missing required fields.
    BadCommitMetadata(),
    /// The workspace has no commits yet.
    NoHead,
}

impl<E: Error + fmt::Debug> fmt::Display for WorkspaceCheckoutError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceCheckoutError::Storage(e) => write!(f, "storage error: {e}"),
            WorkspaceCheckoutError::BadCommitMetadata() => {
                write!(f, "commit metadata missing content field")
            }
            WorkspaceCheckoutError::NoHead => write!(f, "workspace has no commits"),
        }
    }
}

impl<E: Error + fmt::Debug> Error for WorkspaceCheckoutError<E> {}

fn collect_reachable<Blobs: BlobStore<Blake3>>(
    ws: &mut Workspace<Blobs>,
    from: CommitHandle,
) -> Result<
    CommitSet,
    WorkspaceCheckoutError<<Blobs::Reader as BlobStoreGet<Blake3>>::GetError<UnarchiveError>>,
> {
    let mut visited = HashSet::new();
    let mut stack = vec![from];
    let mut result = CommitSet::new();

    while let Some(commit) = stack.pop() {
        if !visited.insert(commit) {
            continue;
        }
        result.insert(&Entry::new(&commit.raw));

        let meta: TribleSet = ws
            .local_blobs
            .reader()
            .unwrap()
            .get(commit)
            .or_else(|_| ws.base_blobs.get(commit))
            .map_err(WorkspaceCheckoutError::Storage)?;

        for (p,) in find!((p: Value<_>), pattern!(&meta, [{ parent: p }])) {
            stack.push(p);
        }
    }

    Ok(result)
}