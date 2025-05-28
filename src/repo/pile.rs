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

use anybytes::Bytes;
use anyhow::Ok;
use hex_literal::hex;
use reft_light::{Apply, ReadHandle, WriteHandle};
use memmap2::MmapOptions;
use std::convert::Infallible;
use std::fs::{File, OpenOptions};
use std::ops::Bound;
use std::path::Path;
use std::ptr::slice_from_raw_parts;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, io::Write};
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::blob::schemas::UnknownBlob;
use crate::blob::{Blob, BlobSchema, FromBlob, ToBlob};
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
            state: Arc::new(validation.map(|state| OnceLock::from(state)).unwrap_or_default()),
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
pub struct PileSwap<H: HashProtocol = Blake3> {
    blobs: BTreeMap<Value<Hash<H>>, IndexEntry>,
    branches: BTreeMap<Id, Value<Handle<H, SimpleArchive>>>,
}

pub struct PileAux<const MAX_PILE_SIZE: usize> {
    pending_length: usize,
    applied_length: usize,
    file: File,
    mmap: Arc<memmap2::MmapRaw>,
}

fn new_length_and_padding(current_length: usize, blob_size: usize) -> (usize, usize) {
    let padding = 64 - (blob_size % 64);
    let new_length = current_length + 64 + blob_size + padding;
    (new_length, padding)
}

impl<const MAX_PILE_SIZE: usize, H: HashProtocol> Apply<PileSwap<H>, PileAux<MAX_PILE_SIZE>> for PileBlobStoreOps<H> {
    fn apply_first(&mut self, first: &mut PileSwap<H>, _second: &PileSwap<H>, auxiliary: &mut PileAux<MAX_PILE_SIZE>) {
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

                auxiliary.file.write_all(header.as_bytes()).expect("failed to write header");
                auxiliary.file.write_all(bytes).expect("failed to write blob bytes");
                auxiliary.file.write_all(&[0; 64][0..padding]).expect("failed to write padding");

                let written_bytes = unsafe {
                    let written_slice =
                        slice_from_raw_parts(auxiliary.mmap.as_ptr().offset(old_length as _), bytes.len())
                            .as_ref()
                            .unwrap();
                    Bytes::from_raw_parts(written_slice, auxiliary.mmap.clone())
                };

                first.blobs.insert(
                    *hash,
                    IndexEntry {
                        state: Arc::new(OnceLock::from(ValidationState::Validated)),
                        bytes: written_bytes.clone(),
                    }
                );
            }
        }
    }

    fn apply_second(self, first: &PileSwap<H>, second: &mut PileSwap<H>, _auxiliary: &mut PileAux<MAX_PILE_SIZE>) {
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

pub struct Pile<const MAX_PILE_SIZE: usize, H: HashProtocol = Blake3> {
    w_handle: WriteHandle<PileBlobStoreOps<H>, PileSwap<H>, PileAux<MAX_PILE_SIZE>>,
}

#[derive(Debug, Clone)]
pub struct PileReader<H: HashProtocol> {
    r_handle: ReadHandle<PileSwap<H>>,
}

impl<H: HashProtocol> PileReader<H> {
    pub fn new(r_handle: ReadHandle<PileSwap<H>>) -> Self {
        Self { r_handle }
    }

    pub fn iter(&self) -> PileBlobStoreIter<H> {
        PileBlobStoreIter {
            read_handle: self.r_handle.clone(),
            cursor: None,
        }
    }
}

impl<H> BlobStoreGetOp<H> for PileReader<H>
where
    H: HashProtocol,
{
    type Err = GetBlobError;

    fn get<T, S>(&self, handle: Value<Handle<H, S>>) -> Result<T, Self::Err>
    where
        S: BlobSchema + 'static,
        T: FromBlob<S>,
        Handle<H, S>: ValueSchema,
    {
        let hash: &Value<Hash<H>> = handle.as_transmute();

        let Some(r_handle) = self.r_handle.enter() else {
            return Err(GetBlobError::BlobNotFound);
            // TODO: Maybe we should return a different error here?
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
                return Ok(Blob::new(entry.bytes.clone()).from_blob());
            }
            ValidationState::Invalid => {
                return Err(GetBlobError::ValidationError(entry.bytes.clone()));
            }
        }
    }
}

impl<H: HashProtocol, const MAX_PILE_SIZE: usize> BlobStorage<H> for Pile<MAX_PILE_SIZE, H> {
    type Reader = PileReader<H>;

    fn reader(&self) -> Self::Reader {
        PileReader::new(self.w_handle.clone())
    }
}

#[derive(Debug)]
pub enum OpenError {
    IoError(std::io::Error),
    MagicMarkerError,
    HeaderError,
    UnexpectedEndOfFile,
    FileLengthError,
    PileTooLarge,
}

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

pub enum UpdateBranchError<H: HashProtocol> {
    PreviousHashMismatch(Option<Value<Handle<H, SimpleArchive>>>),
    IoError(std::io::Error),
    PileTooLarge,
}

impl<H: HashProtocol> std::error::Error for UpdateBranchError<H> {}

unsafe impl<H: HashProtocol> Send for UpdateBranchError<H> {}
unsafe impl<H: HashProtocol> Sync for UpdateBranchError<H> {}

impl<H: HashProtocol> std::fmt::Debug for UpdateBranchError<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateBranchError::PreviousHashMismatch(old) => {
                write!(f, "Previous hash mismatch: {:?}", old)
            }
            UpdateBranchError::IoError(err) => write!(f, "IO error: {}", err),
            UpdateBranchError::PileTooLarge => write!(f, "Pile too large"),
        }
    }
}

impl<H: HashProtocol> std::fmt::Display for UpdateBranchError<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateBranchError::PreviousHashMismatch(old) => {
                write!(f, "Previous hash mismatch: {:?}", old)
            }
            UpdateBranchError::IoError(err) => write!(f, "IO error: {}", err),
            UpdateBranchError::PileTooLarge => write!(f, "Pile too large"),
        }
    }
}

impl<H: HashProtocol> From<std::io::Error> for UpdateBranchError<H> {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

#[derive(Debug)]
pub enum GetBlobError {
    BlobNotFound,
    ValidationError(Bytes),
}

impl std::fmt::Display for GetBlobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetBlobError::BlobNotFound => write!(f, "Blob not found"),
            GetBlobError::ValidationError(_) => write!(f, "Validation error"),
        }
    }
}

impl std::error::Error for GetBlobError {}

#[derive(Debug)]
pub enum FlushError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for FlushError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

//TODO Handle incomplete writes by truncating the file
//TODO Add the ability to skip corrupted blobs
impl<const MAX_PILE_SIZE: usize, H: HashProtocol> Pile<MAX_PILE_SIZE, H> {
    pub fn open(path: &Path) -> Result<Self, OpenError> {
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
            let written_slice = slice_from_raw_parts(mmap.as_ptr(), length)
                .as_ref()
                .unwrap();
            Bytes::from_raw_parts(written_slice, mmap.clone())
        };
        if bytes.len() % 64 != 0 {
            return Err(OpenError::FileLengthError);
        }

        let mut blobs = BTreeMap::new();
        let mut branches = BTreeMap::new();

        while bytes.len() > 0 {
            if bytes.len() < 16 {
                return Err(OpenError::UnexpectedEndOfFile);
            }
            let magic = bytes[0..16].try_into().unwrap();
            match magic {
                MAGIC_MARKER_BLOB => {
                    let Ok(header) = bytes.view_prefix::<BlobHeader>() else {
                        return Err(OpenError::HeaderError);
                    };
                    let hash = Value::new(header.hash);
                    let length = header.length as usize;
                    let Some(blob_bytes) = bytes.take_prefix(length) else {
                        return Err(OpenError::UnexpectedEndOfFile);
                    };

                    let Some(_) = bytes.take_prefix(64 - (length % 64)) else {
                        return Err(OpenError::UnexpectedEndOfFile);
                    };

                    blobs.insert(hash, IndexEntry::new(blob_bytes, None));
                }
                MAGIC_MARKER_BRANCH => {
                    let Ok(header) = bytes.view_prefix::<BranchHeader>() else {
                        return Err(OpenError::HeaderError);
                    };
                    let Some(branch_id) = Id::new(header.branch_id) else {
                        return Err(OpenError::HeaderError);
                    };
                    let hash = Value::new(header.hash);
                    branches.insert(branch_id, hash);
                }
                _ => return Err(OpenError::MagicMarkerError),
            };
        }

        Ok(Self {
            w_handle: reft_light::new(
                PileSwap {
                    blobs,
                    branches,
                },
                PileAux {
                    pending_length: length,
                    applied_length: length,
                    file,
                    mmap,
                },
            ),
        })
    }

    pub fn flush(&self) -> Result<(), FlushError> {
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

use super::{BlobStorage, BlobStoreGetOp, BlobStoreListOp, BlobStorePutOp, BranchStore, PushResult};

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
            read_handle.blobs.range((Bound::Excluded(cursor), Bound::Unbounded))
        } else {
            // If we don't have a cursor, we start from the beginning.
            read_handle.blobs.range((
                Bound::Unbounded::<Value<Hash<H>>>,
                Bound::Unbounded,
            ))
        };

        let (hash, entry) = iter.next()?;
        self.cursor = Some(*hash);

        let bytes = entry.bytes.clone();
        return Some(((*hash).into(), Blob::new(bytes)));
        //TODO we may want to use batching in the future to gain more performance and amortize
        // the cost of creating the iterator over the BTreeMap.
    }
}

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

impl<H> BlobStoreListOp<H> for PileReader<H>
where
    H: HashProtocol,
{
    type Err = Infallible;
    type Iter<'a> = PileBlobStoreListIter<H>;

    fn list(&self) -> Self::Iter<'static> {
        PileBlobStoreListIter {
            inner: self.iter(),
        }
    }
}

impl<const MAX_PILE_SIZE: usize, H> BlobStorePutOp<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = InsertError;

    fn put<S, T>(&mut self, item: T) -> Result<Value<Handle<H, S>>, Self::Err>
    where
        S: BlobSchema + 'static,
        T: ToBlob<S>,
        Handle<H, S>: ValueSchema
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

pub struct PileBranchStoreIter<H>
where
    H: HashProtocol,
{
    read_handle: ReadHandle<PileSwap<H>>,
    cursor: Option<Id>,
}

impl<'a, H> Iterator for PileBranchStoreIter<H>
where
    H: HashProtocol,
{
    type Item = Result<Id, Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        let read_handle = self.read_handle.enter()?;
        let mut iter = if let Some(cursor) = self.cursor.take() {
            // If we have a cursor, we start from the cursor.
            // We use `Bound::Excluded` to skip the cursor itself.
            read_handle.branches.range((Bound::Excluded(cursor), Bound::Unbounded))
        } else {
            // If we don't have a cursor, we start from the beginning.
            read_handle.branches.range((
                Bound::Unbounded::<Id>,
                Bound::Unbounded,
            ))
        };

        let (id, entry) = iter.next()?;
        Some(Ok(*id))
        //TODO we may want to use batching in the future to gain more performance and amortize
        // the cost of creating the iterator over the BTreeMap.
    }
}

impl<const MAX_PILE_SIZE: usize, H> BranchStore<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type ListErr = std::convert::Infallible;
    type PullErr = std::convert::Infallible;
    type PushErr = UpdateBranchError<H>;

    type ListIter<'a> = PileBranchStoreIter<H>;

    fn list(&self) -> Self::ListIter<'static> {
        PileBranchStoreIter {
            read_handle: self.w_handle.clone(),
            cursor: None,
        }
    }

    fn get(&self, id: Id) -> Result<Option<Value<Handle<H, SimpleArchive>>>, Self::PullErr> {
        self.branches.get(&branch_id).copied()

        Ok(self.get_branch(id))
    }
    pub fn get_branch(&self, branch_id: Id) -> Option<Value<Handle<H, SimpleArchive>>> {
    }
    fn put(
        &mut self,
        id: Id,
        old: Option<Value<Handle<H, SimpleArchive>>>,
        new: Value<Handle<H, SimpleArchive>>,
    ) -> Result<super::PushResult<H>, Self::PushErr> {
        let result = self.update_branch(id, old, new);
        match result {
            Ok(()) => Ok(PushResult::Success()),
            Err(UpdateBranchError::PreviousHashMismatch(old)) => Ok(PushResult::Conflict(old)),
            Err(err) => Err(err),
        }
    }

        pub fn update_branch(
        &mut self,
        branch_id: Id,
        old: Option<Value<Handle<H, SimpleArchive>>>,
        new: Value<Handle<H, SimpleArchive>>,
    ) -> Result<(), UpdateBranchError<H>> {
        {
            let current_hash = self.branches.get(&branch_id);
            if current_hash != old.as_ref() {
                return Err(UpdateBranchError::PreviousHashMismatch(
                    current_hash.cloned(),
                ));
            }
        }

        let new_length = self.length + 64;
        if new_length > MAX_PILE_SIZE {
            return Err(UpdateBranchError::PileTooLarge);
        }

        self.length = new_length;

        let header = BranchHeader::new(branch_id, new);

        self.file.write_all(header.as_bytes())?;

        self.branches.insert(branch_id, new);

        Ok(())
    }

    pub fn force_branch(
        &mut self,
        branch_id: Id,
        hash: Value<Handle<H, SimpleArchive>>,
    ) -> Result<(), InsertError> {
        let new_length = self.length + 64;
        if new_length > MAX_PILE_SIZE {
            return Err(InsertError::PileTooLarge);
        }

        self.length = new_length;

        let header = BranchHeader::new(branch_id, hash);

        self.file.write_all(header.as_bytes())?;

        self.branches.insert(branch_id, hash);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::RngCore;
    use tempfile;

    #[test]
    fn open() {
        const RECORD_LEN: usize = 1 << 10; // 1k
        const RECORD_COUNT: usize = 1 << 20; // 1M
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
}
