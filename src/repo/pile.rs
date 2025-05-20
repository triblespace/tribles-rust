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
use hex_literal::hex;
use memmap2::MmapOptions;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::ptr::slice_from_raw_parts;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, io::Write};
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::blob::schemas::UnknownBlob;
use crate::blob::{Blob, BlobSchema};
use crate::id::{Id, RawId};
use crate::prelude::blobschemas::SimpleArchive;
use crate::prelude::valueschemas::Handle;
use crate::value::schemas::hash::{Blake3, Hash, HashProtocol};
use crate::value::{RawValue, Value};

const MAGIC_MARKER_BLOB: RawId = hex!("1E08B022FF2F47B6EBACF1D68EB35D96");
const MAGIC_MARKER_BRANCH: RawId = hex!("2BC991A7F5D5D2A3A468C53B0AA03504");

#[derive(Debug, Clone, Copy)]
enum ValidationState {
    Unvalidated,
    Validated,
    Invalid,
}

struct IndexEntry {
    bytes: Bytes,
    state: ValidationState,
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

pub struct Pile<const MAX_PILE_SIZE: usize, H: HashProtocol = Blake3> {
    file: File,
    length: usize,
    mmap: Arc<memmap2::MmapRaw>,
    index: HashMap<Value<Hash<H>>, Mutex<IndexEntry>>,
    branches: HashMap<Id, Value<Handle<H, SimpleArchive>>>,
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
    PoisonError,
    PileTooLarge,
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::IoError(err) => write!(f, "IO error: {}", err),
            InsertError::PoisonError => write!(f, "Poison error"),
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

impl<T> From<PoisonError<T>> for InsertError {
    fn from(_err: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

pub enum UpdateBranchError<H: HashProtocol> {
    PreviousHashMismatch(Option<Value<Handle<H, SimpleArchive>>>),
    IoError(std::io::Error),
    PoisonError,
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
            UpdateBranchError::PoisonError => write!(f, "Poison error"),
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
            UpdateBranchError::PoisonError => write!(f, "Poison error"),
            UpdateBranchError::PileTooLarge => write!(f, "Pile too large"),
        }
    }
}

impl<H: HashProtocol> From<std::io::Error> for UpdateBranchError<H> {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl<T, H: HashProtocol> From<PoisonError<T>> for UpdateBranchError<H> {
    fn from(_err: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

#[derive(Debug)]
pub enum GetBlobError {
    BlobNotFound,
    PoisonError,
    ValidationError(Bytes),
}

impl std::fmt::Display for GetBlobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetBlobError::BlobNotFound => write!(f, "Blob not found"),
            GetBlobError::PoisonError => write!(f, "Poison error"),
            GetBlobError::ValidationError(_) => write!(f, "Validation error"),
        }
    }
}

impl std::error::Error for GetBlobError {}

impl<T> From<PoisonError<T>> for GetBlobError {
    fn from(_err: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

#[derive(Debug)]
pub enum FlushError {
    IoError(std::io::Error),
    PoisonError,
}

impl From<std::io::Error> for FlushError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl<T> From<PoisonError<T>> for FlushError {
    fn from(_err: PoisonError<T>) -> Self {
        Self::PoisonError
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

        let mut index = HashMap::new();
        let mut branches = HashMap::new();

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

                    let blob = IndexEntry {
                        state: ValidationState::Unvalidated,
                        bytes: blob_bytes,
                    };
                    index.insert(hash, Mutex::new(blob));
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
            file,
            length,
            mmap,
            index,
            branches,
        })
    }

    #[must_use]
    fn insert_blob_raw(
        &mut self,
        hash: Value<Hash<H>>,
        validation: ValidationState,
        bytes: &Bytes,
    ) -> Result<Bytes, InsertError> {
        let old_length = self.length;
        let padding = 64 - (bytes.len() % 64);

        let new_length = old_length + 64 + bytes.len() + padding;
        if new_length > MAX_PILE_SIZE {
            return Err(InsertError::PileTooLarge);
        }

        self.length = new_length;

        let now_in_sys = SystemTime::now();
        let now_since_epoch = now_in_sys
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards");
        let now_in_ms = now_since_epoch.as_millis();

        let header = BlobHeader::new(now_in_ms as u64, bytes.len() as u64, hash);

        self.file.write_all(header.as_bytes())?;
        self.file.write_all(bytes)?;
        self.file.write_all(&[0; 64][0..padding])?;

        let written_bytes = unsafe {
            let written_slice =
                slice_from_raw_parts(self.mmap.as_ptr().offset(old_length as _), bytes.len())
                    .as_ref()
                    .unwrap();
            Bytes::from_raw_parts(written_slice, self.mmap.clone())
        };

        self.index.insert(
            hash,
            Mutex::new(IndexEntry {
                state: validation,
                bytes: written_bytes.clone(),
            }),
        );

        Ok(written_bytes)
    }

    pub fn insert_blob<T: BlobSchema>(
        &mut self,
        blob: Blob<T>,
    ) -> Result<Value<Hash<H>>, InsertError> {
        let handle: Value<Handle<H, T>> = blob.get_handle();
        let hash = handle.into();

        let bytes = &blob.bytes;
        let _on_disk_bytes = self.insert_blob_raw(hash, ValidationState::Validated, bytes)?;

        Ok(hash)
    }

    pub fn insert_blob_validated(
        &mut self,
        hash: Value<Hash<H>>,
        value: &Bytes,
    ) -> Result<Bytes, InsertError> {
        self.insert_blob_raw(hash, ValidationState::Validated, value)
    }

    pub fn insert_blob_unvalidated(
        &mut self,
        hash: Value<Hash<H>>,
        value: &Bytes,
    ) -> Result<Bytes, InsertError> {
        self.insert_blob_raw(hash, ValidationState::Unvalidated, value)
    }

    pub fn get_blob<T: BlobSchema>(
        &self,
        handle: &Value<Handle<H, T>>,
    ) -> Result<Blob<T>, GetBlobError> {
        let hash: &Value<Hash<H>> = handle.as_transmute();
        let Some(blob) = self.index.get(hash) else {
            return Err(GetBlobError::BlobNotFound);
        };
        let mut entry = blob.lock().unwrap();
        match entry.state {
            ValidationState::Validated => {
                return Ok(Blob::new(entry.bytes.clone()));
            }
            ValidationState::Invalid => {
                return Err(GetBlobError::ValidationError(entry.bytes.clone()));
            }
            ValidationState::Unvalidated => {
                let computed_hash = Hash::<H>::digest(&entry.bytes);
                if computed_hash != *hash {
                    entry.state = ValidationState::Invalid;
                    return Err(GetBlobError::ValidationError(entry.bytes.clone()));
                } else {
                    entry.state = ValidationState::Validated;
                    return Ok(Blob::new(entry.bytes.clone()));
                }
            }
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

    pub fn get_branch(&self, branch_id: Id) -> Option<Value<Handle<H, SimpleArchive>>> {
        self.branches.get(&branch_id).copied()
    }

    pub fn flush(&self) -> Result<(), FlushError> {
        self.file.sync_data()?;
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

use super::{BlobStorage, BlobStoreGetOp, BlobStoreListOp, BlobStorePutOp, BranchRepo, PushResult};

pub struct PileBlobIterator<'a, H: HashProtocol> {
    iter: std::collections::hash_map::Keys<'a, Value<Hash<H>>, Mutex<IndexEntry>>,
}

impl<'a, H: HashProtocol> Iterator for PileBlobIterator<'a, H> {
    type Item = Result<Value<Handle<H, UnknownBlob>>, std::convert::Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|hash| Ok(hash.transmute()))
    }
}

impl<const MAX_PILE_SIZE: usize, H> BlobStoreListOp<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = std::convert::Infallible;
    type Iter<'a> = PileBlobIterator<'a, H>;

    fn list<'a>(&'a self) -> Self::Iter<'a> {
        PileBlobIterator {
            iter: self.index.keys(),
        }
    }
}

impl<const MAX_PILE_SIZE: usize, H> BlobStoreGetOp<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = GetBlobError;

    fn get<T>(
        &self,
        hash: Value<crate::prelude::valueschemas::Handle<H, T>>,
    ) -> Result<Blob<T>, Self::Err>
    where
        T: crate::prelude::BlobSchema + 'static,
    {
        self.get_blob(&hash)
    }
}

impl<const MAX_PILE_SIZE: usize, H> BlobStorePutOp<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = InsertError;

    fn put<T: BlobSchema>(
        &mut self,
        blob: Blob<T>,
    ) -> Result<Value<Handle<H, T>>, <Self as BlobStorePutOp<H>>::Err>
    where
        T: crate::prelude::BlobSchema + 'static,
    {
        let hash = self.insert_blob(blob)?;
        Ok(hash.transmute())
    }
}

impl<const MAX_PILE_SIZE: usize, H> BlobStorage<H> for Pile<MAX_PILE_SIZE, H> where H: HashProtocol {}

pub struct PileBranchIterator<'a, H: HashProtocol> {
    iter: std::collections::hash_map::Keys<'a, Id, Value<Handle<H, SimpleArchive>>>,
}

impl<'a, H: HashProtocol> Iterator for PileBranchIterator<'a, H> {
    type Item = Result<Id, std::convert::Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|id| Ok(*id))
    }
}

impl<const MAX_PILE_SIZE: usize, H> BranchRepo<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type ListErr = std::convert::Infallible;
    type PullErr = std::convert::Infallible;
    type PushErr = UpdateBranchError<H>;

    type ListIter<'a> = PileBranchIterator<'a, H>;

    fn list<'a>(&'a self) -> Self::ListIter<'a> {
        PileBranchIterator {
            iter: self.branches.keys(),
        }
    }

    fn pull(&self, id: Id) -> Result<Option<Value<Handle<H, SimpleArchive>>>, Self::PullErr> {
        Ok(self.get_branch(id))
    }

    fn push(
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
            pile.insert_blob(data).unwrap();
        });

        pile.flush().unwrap();

        drop(pile);

        let _pile: Pile<MAX_PILE_SIZE> = Pile::open(&tmp_pile).unwrap();
    }
}
