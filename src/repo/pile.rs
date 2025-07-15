//! A Pile is a collection of Blobs and Branches stored in a single file,
//! it is designed to be used as a local storage for a repository.
//! It is append-only for durability and memory-mapped for fast access.
//! Blobs are stored in the file as-is, and are identified by their hash.
//! Branches are stored in the file as a mapping from a branch id to a blob hash.
//! It can safely be shared between threads.
//!
//! # File Format
//! ## Blob Storage
//! ```text
//!                              8 byte  8 byte
//!             ┌────16 byte───┐┌──────┐┌──────┐┌────────────32 byte───────────┐
//!           ┌ ┌──────────────┐┌──────┐┌──────┐┌──────────────────────────────┐
//!  header   │ │magic number A││ time ││length││             hash             │
//!           └ └──────────────┘└──────┘└──────┘└──────────────────────────────┘
//!             ┌────────────────────────────64 byte───────────────────────────┐
//!           ┌ ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┐
//!           │ │                                                              │
//!  payload  │ │              bytes (64byte aligned and padded)               │
//!           │ │                                                              │
//!           └ └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┘
//! ```
//!
//! ## Branch Storage
//! ```text
//!             ┌────16 byte───┐┌────16 byte───┐┌────────────32 byte───────────┐
//!           ┌ ┌──────────────┐┌──────────────┐┌──────────────────────────────┐
//!  header   │ │magic number B││  branch id   ││             hash             │
//!           └ └──────────────┘└──────────────┘└──────────────────────────────┘
//! ```
//!
//! A `Pile` stores blobs and branches sequentially in a single append-only file.
//! Each record begins with a 16 byte magic marker that identifies whether the
//! block is a blob or a branch. Blob headers additionally contain a timestamp,
//! the byte length of the payload and the hash of the blob. Branch headers
//! contain the branch id and the referenced blob hash.
//!
//! When opening a file, [`Pile::try_open`] validates that every block header
//! uses one of the known markers and that the entire block fits into the file.
//! It does **not** verify any hashes. If a record is truncated or has an unknown
//! marker, the function returns [`OpenError::CorruptPile { valid_length }`] where
//! `valid_length` marks the number of bytes that belong to well formed blocks.
//!
//! [`Pile::open`] provides a convenience wrapper that attempts the same parsing
//! but truncates the file to `valid_length` whenever such a corruption error is
//! encountered. This recovers from interrupted writes by discarding incomplete
//! bytes so that the file can still be used.
//!
//! Hash verification happens lazily when individual blobs are loaded, keeping
//! the initial opening cost low.

use anybytes::Bytes;
use hex_literal::hex;
use memmap2::MmapOptions;
use reft_light::{Apply, ReadHandle, WriteHandle};
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::ops::Bound;
use std::path::Path;
use std::ptr::slice_from_raw_parts;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
};
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::blob::schemas::UnknownBlob;
use crate::blob::{Blob, BlobSchema, ToBlob, TryFromBlob};
use crate::id::{Id, RawId};
use crate::prelude::blobschemas::SimpleArchive;
use crate::prelude::valueschemas::Handle;
use crate::value::schemas::hash::{Blake3, Hash, HashProtocol};
use crate::value::{RawValue, Value, ValueSchema};

const MAGIC_MARKER_BLOB: RawId = hex!("1E08B022FF2F47B6EBACF1D68EB35D96");
const MAGIC_MARKER_BRANCH: RawId = hex!("2BC991A7F5D5D2A3A468C53B0AA03504");

enum PileBlobStoreOps<H: HashProtocol> {
    Insert(Value<Hash<H>>, Bytes),
}

#[derive(Debug, Clone, Copy)]
pub enum ValidationState {
    Validated,
    Invalid,
}

#[derive(Debug, Clone)]
struct IndexEntry {
    state: Arc<OnceLock<ValidationState>>,
    bytes: Bytes,
}

impl IndexEntry {
    fn new(bytes: Bytes, validation: Option<ValidationState>) -> Self {
        Self {
            state: Arc::new(
                validation
                    .map(|state| OnceLock::from(state))
                    .unwrap_or_default(),
            ),
            bytes,
        }
    }
}

#[derive(TryFromBytes, IntoBytes, Immutable, KnownLayout, Copy, Clone)]
#[repr(C)]
struct BranchHeader {
    magic_marker: RawId,
    branch_id: RawId,
    hash: RawValue,
}

impl BranchHeader {
    fn new<H: HashProtocol>(branch_id: Id, hash: Value<Handle<H, SimpleArchive>>) -> Self {
        Self {
            magic_marker: MAGIC_MARKER_BRANCH,
            branch_id: *branch_id,
            hash: hash.raw,
        }
    }
}

#[derive(TryFromBytes, IntoBytes, Immutable, KnownLayout, Copy, Clone)]
#[repr(C)]
struct BlobHeader {
    magic_marker: RawId,
    timestamp: u64,
    length: u64,
    hash: RawValue,
}

impl BlobHeader {
    fn new<H: HashProtocol>(timestamp: u64, length: u64, hash: Value<Hash<H>>) -> Self {
        Self {
            magic_marker: MAGIC_MARKER_BLOB,
            timestamp,
            length,
            hash: hash.raw,
        }
    }
}

#[derive(Debug, Clone)]
/// In-memory view of the on-disk pile used while applying write operations.
///
/// `PileSwap` mirrors the index portion of the pile file so that new blobs can
/// be staged before being flushed to disk.
pub(crate) struct PileSwap<H: HashProtocol> {
    blobs: BTreeMap<Value<Hash<H>>, IndexEntry>,
}

/// Additional state kept alongside [`PileSwap`] while writing to the pile.
///
/// It tracks the current file handle, memory mapping and pending write lengths
/// to enforce the maximum pile size.
pub(crate) struct PileAux<const MAX_PILE_SIZE: usize, H: HashProtocol> {
    pending_length: usize,
    applied_length: usize,
    file: File,
    mmap: Arc<memmap2::MmapRaw>,
    branches: HashMap<Id, Value<Handle<H, SimpleArchive>>>,
}

fn new_length_and_padding(current_length: usize, blob_size: usize) -> (usize, usize) {
    let padding = (64 - (blob_size % 64)) % 64;
    let new_length = current_length + 64 + blob_size + padding;
    (new_length, padding)
}

impl<const MAX_PILE_SIZE: usize, H: HashProtocol> Apply<PileSwap<H>, PileAux<MAX_PILE_SIZE, H>>
    for PileBlobStoreOps<H>
{
    fn apply_first(
        &mut self,
        first: &mut PileSwap<H>,
        _second: &PileSwap<H>,
        auxiliary: &mut PileAux<MAX_PILE_SIZE, H>,
    ) {
        match self {
            PileBlobStoreOps::Insert(hash, bytes) => {
                let old_length = auxiliary.applied_length;
                let (new_length, padding) = new_length_and_padding(old_length, bytes.len());

                // This should never happen, because we check the length before appending the operation.
                assert!(new_length <= MAX_PILE_SIZE);

                auxiliary.applied_length = new_length;

                let now_in_sys = SystemTime::now();
                let now_since_epoch = now_in_sys
                    .duration_since(UNIX_EPOCH)
                    .expect("time went backwards");
                let now_in_ms = now_since_epoch.as_millis();

                let header = BlobHeader::new(now_in_ms as u64, bytes.len() as u64, *hash);

                auxiliary
                    .file
                    .write_all(header.as_bytes())
                    .expect("failed to write header");
                auxiliary
                    .file
                    .write_all(bytes)
                    .expect("failed to write blob bytes");
                auxiliary
                    .file
                    .write_all(&[0; 64][0..padding])
                    .expect("failed to write padding");

                let written_bytes = unsafe {
                    let written_slice = slice_from_raw_parts(
                        auxiliary.mmap.as_ptr().offset(old_length as _),
                        bytes.len(),
                    )
                    .as_ref()
                    .unwrap();
                    Bytes::from_raw_parts(written_slice, auxiliary.mmap.clone())
                };

                first.blobs.insert(
                    *hash,
                    IndexEntry {
                        state: Arc::new(OnceLock::from(ValidationState::Validated)),
                        bytes: written_bytes.clone(),
                    },
                );
            }
        }
    }

    fn apply_second(
        self,
        first: &PileSwap<H>,
        second: &mut PileSwap<H>,
        _auxiliary: &mut PileAux<MAX_PILE_SIZE, H>,
    ) {
        match self {
            PileBlobStoreOps::Insert(hash, _blob) => {
                // This operation is idempotent, so we can just
                // ignore it if the blob is already present.

                let first = first.blobs.get(&hash).expect("handle must exist in first");
                second.blobs.entry(hash).or_insert_with(|| IndexEntry {
                    state: first.state.clone(),
                    bytes: first.bytes.clone(),
                });
            }
        }
    }
}

/// A grow-only collection of blobs and branch pointers backed by a single file on disk.
///
/// The pile acts as an append-only log where new blobs or branch updates are appended
/// while an in-memory index is kept for fast retrieval.
pub struct Pile<const MAX_PILE_SIZE: usize, H: HashProtocol = Blake3> {
    w_handle: WriteHandle<PileBlobStoreOps<H>, PileSwap<H>, PileAux<MAX_PILE_SIZE, H>>,
}

impl<const MAX_PILE_SIZE: usize, H> fmt::Debug for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pile").finish()
    }
}

#[derive(Debug, Clone)]
/// Read-only handle referencing a [`Pile`].
///
/// Multiple `PileReader` instances can coexist and provide concurrent access to
/// the same underlying pile data.
pub struct PileReader<H: HashProtocol> {
    r_handle: ReadHandle<PileSwap<H>>,
}

impl<H> PartialEq for PileReader<H>
where
    H: HashProtocol,
{
    fn eq(&self, other: &Self) -> bool {
        self.r_handle == other.r_handle
    }
}

impl<H> Eq for PileReader<H> where H: HashProtocol {}

impl<H: HashProtocol> PileReader<H> {
    /// Creates a new reader from the given handle.
    pub(crate) fn new(r_handle: ReadHandle<PileSwap<H>>) -> Self {
        Self { r_handle }
    }

    /// Returns an iterator over all blobs currently stored in the pile.
    pub fn iter(&self) -> PileBlobStoreIter<H> {
        PileBlobStoreIter {
            read_handle: self.r_handle.clone(),
            cursor: None,
        }
    }
}

impl<H> BlobStoreGet<H> for PileReader<H>
where
    H: HashProtocol,
{
    type GetError<E: Error> = GetBlobError<E>;

    fn get<T, S>(
        &self,
        handle: Value<Handle<H, S>>,
    ) -> Result<T, Self::GetError<<T as TryFromBlob<S>>::Error>>
    where
        S: BlobSchema + 'static,
        T: TryFromBlob<S>,
        Handle<H, S>: ValueSchema,
    {
        let hash: &Value<Hash<H>> = handle.as_transmute();

        let Some(r_handle) = self.r_handle.enter() else {
            return Err(GetBlobError::BlobNotFound);
        };
        let Some(entry) = r_handle.blobs.get(hash) else {
            return Err(GetBlobError::BlobNotFound);
        };
        let state = entry.state.get_or_init(|| {
            let computed_hash = Hash::<H>::digest(&entry.bytes);
            if computed_hash == *hash {
                ValidationState::Validated
            } else {
                ValidationState::Invalid
            }
        });
        match state {
            ValidationState::Validated => {
                let blob: Blob<S> = Blob::new(entry.bytes.clone());
                match blob.try_from_blob() {
                    Ok(value) => return Ok(value),
                    Err(e) => return Err(GetBlobError::ConversionError(e)),
                }
            }
            ValidationState::Invalid => {
                return Err(GetBlobError::ValidationError(entry.bytes.clone()));
            }
        }
    }
}

impl<H: HashProtocol, const MAX_PILE_SIZE: usize> BlobStore<H> for Pile<MAX_PILE_SIZE, H> {
    type Reader = PileReader<H>;

    fn reader(&mut self) -> Self::Reader {
        PileReader::new(self.w_handle.publish().clone())
    }
}

#[derive(Debug)]
pub enum OpenError {
    IoError(std::io::Error),
    PileTooLarge,
    CorruptPile { valid_length: usize },
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::IoError(err) => write!(f, "IO error: {}", err),
            OpenError::PileTooLarge => write!(f, "Pile too large"),
            OpenError::CorruptPile { valid_length } => {
                write!(f, "Corrupt pile at byte {}", valid_length)
            }
        }
    }
}
impl std::error::Error for OpenError {}

impl From<std::io::Error> for OpenError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

#[derive(Debug)]
pub enum InsertError {
    IoError(std::io::Error),
    PileTooLarge,
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::IoError(err) => write!(f, "IO error: {}", err),
            InsertError::PileTooLarge => write!(f, "Pile too large"),
        }
    }
}
impl std::error::Error for InsertError {}

impl From<std::io::Error> for InsertError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub enum UpdateBranchError {
    IoError(std::io::Error),
    PileTooLarge,
}

impl std::error::Error for UpdateBranchError {}

unsafe impl Send for UpdateBranchError {}
unsafe impl Sync for UpdateBranchError {}

impl std::fmt::Debug for UpdateBranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateBranchError::IoError(err) => write!(f, "IO error: {}", err),
            UpdateBranchError::PileTooLarge => write!(f, "Pile too large"),
        }
    }
}

impl std::fmt::Display for UpdateBranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateBranchError::IoError(err) => write!(f, "IO error: {}", err),
            UpdateBranchError::PileTooLarge => write!(f, "Pile too large"),
        }
    }
}

impl From<std::io::Error> for UpdateBranchError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

#[derive(Debug)]
pub enum GetBlobError<E: Error> {
    BlobNotFound,
    ValidationError(Bytes),
    ConversionError(E),
}

impl<E: Error> std::fmt::Display for GetBlobError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetBlobError::BlobNotFound => write!(f, "Blob not found"),
            GetBlobError::ConversionError(err) => write!(f, "Conversion error: {}", err),
            GetBlobError::ValidationError(_) => write!(f, "Validation error"),
        }
    }
}

impl<E: Error> std::error::Error for GetBlobError<E> {}

#[derive(Debug)]
pub enum FlushError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for FlushError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl<const MAX_PILE_SIZE: usize, H: HashProtocol> Pile<MAX_PILE_SIZE, H> {
    /// Opens an existing pile and truncates any corrupted tail data if found.
    pub fn open(path: &Path) -> Result<Self, OpenError> {
        match Self::try_open(path) {
            Ok(pile) => Ok(pile),
            Err(OpenError::CorruptPile { valid_length }) => {
                // Truncate the file at the first valid offset and try again.
                OpenOptions::new()
                    .write(true)
                    .open(&path)?
                    .set_len(valid_length as u64)?;
                Self::try_open(path)
            }
            Err(err) => Err(err),
        }
    }

    /// Opens a pile file without repairing potential corruption.
    ///
    /// The file is scanned to ensure record boundaries are valid. If a
    /// truncated or malformed record is encountered the function returns
    /// [`OpenError::CorruptPile`] with the length of the valid prefix so the
    /// caller may decide how to handle it.
    pub fn try_open(path: &Path) -> Result<Self, OpenError> {
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)?;
        let length = file.metadata()?.len() as usize;
        if length > MAX_PILE_SIZE {
            return Err(OpenError::PileTooLarge);
        }

        let mmap = MmapOptions::new()
            .len(MAX_PILE_SIZE)
            .map_raw_read_only(&file)?;
        let mmap = Arc::new(mmap);
        let mut bytes = unsafe {
            let slice = slice_from_raw_parts(mmap.as_ptr(), length)
                .as_ref()
                .unwrap();
            Bytes::from_raw_parts(slice, mmap.clone())
        };

        let mut blobs = BTreeMap::new();
        let mut branches = HashMap::new();

        while bytes.len() > 0 {
            let start_offset = length - bytes.len();
            if bytes.len() < 16 {
                return Err(OpenError::CorruptPile {
                    valid_length: start_offset,
                });
            }
            let magic = bytes[0..16].try_into().unwrap();
            match magic {
                MAGIC_MARKER_BLOB => {
                    let Ok(header) = bytes.view_prefix::<BlobHeader>() else {
                        return Err(OpenError::CorruptPile {
                            valid_length: start_offset,
                        });
                    };
                    let data_len = header.length as usize;
                    let pad = (64 - (data_len % 64)) % 64;
                    let hash = Value::new(header.hash);
                    let blob_bytes = bytes.take_prefix(data_len).ok_or(OpenError::CorruptPile {
                        valid_length: start_offset,
                    })?;
                    bytes.take_prefix(pad).ok_or(OpenError::CorruptPile {
                        valid_length: start_offset,
                    })?;
                    blobs.insert(hash, IndexEntry::new(blob_bytes, None));
                }
                MAGIC_MARKER_BRANCH => {
                    let Ok(header) = bytes.view_prefix::<BranchHeader>() else {
                        return Err(OpenError::CorruptPile {
                            valid_length: start_offset,
                        });
                    };
                    let branch_id = Id::new(header.branch_id).ok_or(OpenError::CorruptPile {
                        valid_length: start_offset,
                    })?;
                    let hash = Value::new(header.hash);
                    branches.insert(branch_id, hash);
                }
                _ => {
                    return Err(OpenError::CorruptPile {
                        valid_length: start_offset,
                    })
                }
            }
        }

        Ok(Self {
            w_handle: reft_light::new(
                PileSwap { blobs },
                PileAux {
                    pending_length: length,
                    applied_length: length,
                    file,
                    mmap,
                    branches,
                },
            ),
        })
    }

    /// Persists any queued writes to the underlying pile file.
    pub fn flush(&mut self) -> Result<(), FlushError> {
        self.w_handle.flush();
        self.w_handle.auxiliary().file.sync_data()?;
        Ok(())
    }
}

impl<const MAX_PILE_SIZE: usize, H> Drop for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    fn drop(&mut self) {
        self.flush().unwrap();
    }
}

use super::{BlobStore, BlobStoreGet, BlobStoreList, BlobStorePut, BranchStore, PushResult};

/// Iterator returned by [`PileReader::iter`].
///
/// Iterates over all `(Handle, Blob)` pairs currently stored in the pile.
pub struct PileBlobStoreIter<H>
where
    H: HashProtocol,
{
    read_handle: ReadHandle<PileSwap<H>>,
    cursor: Option<Value<Hash<H>>>,
}

impl<'a, H> Iterator for PileBlobStoreIter<H>
where
    H: HashProtocol,
{
    type Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>);

    fn next(&mut self) -> Option<Self::Item> {
        let read_handle = self.read_handle.enter()?;
        let mut iter = if let Some(cursor) = self.cursor.take() {
            // If we have a cursor, we start from the cursor.
            // We use `Bound::Excluded` to skip the cursor itself.
            read_handle
                .blobs
                .range((Bound::Excluded(cursor), Bound::Unbounded))
        } else {
            // If we don't have a cursor, we start from the beginning.
            read_handle
                .blobs
                .range((Bound::Unbounded::<Value<Hash<H>>>, Bound::Unbounded))
        };

        let (hash, entry) = iter.next()?;
        self.cursor = Some(*hash);

        let bytes = entry.bytes.clone();
        return Some(((*hash).into(), Blob::new(bytes)));
        // Note: we may want to use batching in the future to gain more performance and amortize
        // the cost of creating the iterator over the BTreeMap.
    }
}

/// Adapter over [`PileBlobStoreIter`] that yields only the blob handles.
pub struct PileBlobStoreListIter<H>
where
    H: HashProtocol,
{
    inner: PileBlobStoreIter<H>,
}

impl<H> Iterator for PileBlobStoreListIter<H>
where
    H: HashProtocol,
{
    type Item = Result<Value<Handle<H, UnknownBlob>>, Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        let (handle, _) = self.inner.next()?;
        Some(Ok(handle))
    }
}

impl<H> BlobStoreList<H> for PileReader<H>
where
    H: HashProtocol,
{
    type Err = Infallible;
    type Iter<'a> = PileBlobStoreListIter<H>;

    fn blobs(&self) -> Self::Iter<'static> {
        PileBlobStoreListIter { inner: self.iter() }
    }
}

impl<const MAX_PILE_SIZE: usize, H> BlobStorePut<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type PutError = InsertError;

    fn put<S, T>(&mut self, item: T) -> Result<Value<Handle<H, S>>, Self::PutError>
    where
        S: BlobSchema + 'static,
        T: ToBlob<S>,
        Handle<H, S>: ValueSchema,
    {
        let blob = ToBlob::to_blob(item);

        let aux = self.w_handle.auxiliary_mut();
        let blob_size = blob.bytes.len();
        if aux.pending_length + blob_size + 64 > MAX_PILE_SIZE {
            return Err(InsertError::PileTooLarge);
        }

        aux.pending_length += blob_size + 64;

        let handle: Value<Handle<H, S>> = blob.get_handle();
        let hash = handle.into();

        let bytes = blob.bytes;
        self.w_handle.append(PileBlobStoreOps::Insert(hash, bytes));

        Ok(handle.transmute())
    }
}

/// Iterator over the branch identifiers present in a [`Pile`].
pub struct PileBranchStoreIter<'a, H: HashProtocol> {
    iter: std::collections::hash_map::Keys<'a, Id, Value<Handle<H, SimpleArchive>>>,
}

impl<'a, H: HashProtocol> Iterator for PileBranchStoreIter<'a, H> {
    type Item = Result<Id, std::convert::Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|id| Ok(*id))
    }
}

impl<const MAX_PILE_SIZE: usize, H> BranchStore<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type BranchesError = std::convert::Infallible;
    type HeadError = std::convert::Infallible;
    type UpdateError = UpdateBranchError;

    type ListIter<'a> = PileBranchStoreIter<'a, H>;

    fn branches<'a>(&'a self) -> Self::ListIter<'a> {
        PileBranchStoreIter {
            iter: self.w_handle.auxiliary().branches.keys(),
        }
    }

    fn head(&self, id: Id) -> Result<Option<Value<Handle<H, SimpleArchive>>>, Self::HeadError> {
        Ok(self.w_handle.auxiliary().branches.get(&id).copied())
    }

    fn update(
        &mut self,
        id: Id,
        old: Option<Value<Handle<H, SimpleArchive>>>,
        new: Value<Handle<H, SimpleArchive>>,
    ) -> Result<super::PushResult<H>, Self::UpdateError> {
        let aux = self.w_handle.auxiliary_mut();

        let current_hash = aux.branches.get(&id);
        if current_hash != old.as_ref() {
            return Ok(PushResult::Conflict(current_hash.cloned()));
        }

        let new_length = aux.pending_length + 64;
        if new_length > MAX_PILE_SIZE {
            return Err(UpdateBranchError::PileTooLarge);
        }

        aux.pending_length = new_length;

        let header = BranchHeader::new(id, new);

        aux.file.write_all(header.as_bytes())?;

        aux.branches.insert(id, new);

        Ok(PushResult::Success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::RngCore;
    use std::io::Write;
    use tempfile;

    #[test]
    fn open() {
        const RECORD_LEN: usize = 1 << 10; // 1k
        const RECORD_COUNT: usize = 1 << 12; // 4k
        const MAX_PILE_SIZE: usize = 1 << 30; // 100GB

        let mut rng = rand::thread_rng();
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp_pile = tmp_dir.path().join("test.pile");
        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&tmp_pile).unwrap();

        (0..RECORD_COUNT).for_each(|_| {
            let mut record = Vec::with_capacity(RECORD_LEN);
            rng.fill_bytes(&mut record);

            let data: Blob<UnknownBlob> = Blob::new(Bytes::from_source(record));
            pile.put(data).unwrap();
        });

        pile.flush().unwrap();

        drop(pile);

        let _pile: Pile<MAX_PILE_SIZE> = Pile::open(&tmp_pile).unwrap();
    }

    #[test]
    fn recover_shrink() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        {
            let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
            let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 20]));
            pile.put(blob).unwrap();
            pile.flush().unwrap();
        }

        // Corrupt by removing some bytes from the end
        let file = OpenOptions::new().write(true).open(&path).unwrap();
        let len = file.metadata().unwrap().len();
        file.set_len(len - 10).unwrap();

        let _pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
    }

    #[test]
    fn try_open_corrupt_reports_length() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        {
            let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
            let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 20]));
            pile.put(blob).unwrap();
            pile.flush().unwrap();
        }

        let file_len = std::fs::metadata(&path).unwrap().len();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(file_len - 10)
            .unwrap();

        match Pile::<MAX_PILE_SIZE>::try_open(&path) {
            Err(OpenError::CorruptPile { valid_length }) => assert_eq!(valid_length, 0),
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn open_truncates_unknown_magic() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        {
            let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
            let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 20]));
            pile.put(blob).unwrap();
            pile.flush().unwrap();
        }

        let valid_len = std::fs::metadata(&path).unwrap().len();
        // Append 16 bytes of garbage that don't form a valid marker
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(&[0u8; 16])
            .unwrap();

        let _pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().len(), valid_len);
    }

    #[test]
    fn try_open_partial_header_reports_length() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        {
            let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
            let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 20]));
            pile.put(blob).unwrap();
            pile.flush().unwrap();
        }

        let file_len = std::fs::metadata(&path).unwrap().len();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(file_len + 8)
            .unwrap();

        match Pile::<MAX_PILE_SIZE>::try_open(&path) {
            Err(OpenError::CorruptPile { valid_length }) => {
                assert_eq!(valid_length as u64, file_len)
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    // recover_grow test removed as growth strategy no longer exists
}
