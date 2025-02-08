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
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, io::Write};
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::blob::schemas::UnknownBlob;
use crate::blob::{Blob, BlobSchema};
use crate::id::{Id, RawId};
use crate::prelude::valueschemas::Handle;
use crate::value::schemas::hash::{Blake3, Hash, HashProtocol};
use crate::value::{RawValue, Value};

const MAGIC_MARKER_BLOB: RawId = hex!("1E08B022FF2F47B6EBACF1D68EB35D96");
const MAGIC_MARKER_BRANCH: RawId = hex!("2BC991A7F5D5D2A3A468C53B0AA03504");

struct AppendFile {
    file: File,
    length: usize,
}

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
    fn new<H: HashProtocol>(branch_id: Id, hash: Value<Hash<H>>) -> Self {
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
    file: Mutex<AppendFile>,
    mmap: Arc<memmap2::MmapRaw>,
    index: RwLock<HashMap<Value<Hash<H>>, Mutex<IndexEntry>>>,
    branches: RwLock<HashMap<Id, Value<Hash<H>>>>,
}

#[derive(Debug)]
pub enum LoadError {
    IoError(std::io::Error),
    MagicMarkerError,
    HeaderError,
    UnexpectedEndOfFile,
    FileLengthError,
    PileTooLarge,
}

impl From<std::io::Error> for LoadError {
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

#[derive(Debug)]
pub enum UpdateBranchError<H: HashProtocol> {
    PreviousHashMismatch(Option<Value<Hash<H>>>),
    IoError(std::io::Error),
    PoisonError,
    PileTooLarge,
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
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)?;
        let file_len = file.metadata()?.len() as usize;
        if file_len > MAX_PILE_SIZE {
            return Err(LoadError::PileTooLarge);
        }
        let mmap = MmapOptions::new()
            .len(MAX_PILE_SIZE)
            .map_raw_read_only(&file)?;
        let mmap = Arc::new(mmap);
        let mut bytes = unsafe {
            let written_slice = slice_from_raw_parts(mmap.as_ptr(), file_len)
                .as_ref()
                .unwrap();
            Bytes::from_raw_parts(written_slice, mmap.clone())
        };
        if bytes.len() % 64 != 0 {
            return Err(LoadError::FileLengthError);
        }

        let mut index = HashMap::new();
        let mut branches = HashMap::new();

        while bytes.len() > 0 {
            if bytes.len() < 16 {
                return Err(LoadError::UnexpectedEndOfFile);
            }
            let magic = bytes[0..16].try_into().unwrap();
            match magic {
                MAGIC_MARKER_BLOB => {
                    let Ok(header) = bytes.view_prefix::<BlobHeader>() else {
                        return Err(LoadError::HeaderError);
                    };
                    let hash = Value::new(header.hash);
                    let length = header.length as usize;
                    let Some(blob_bytes) = bytes.take_prefix(length) else {
                        return Err(LoadError::UnexpectedEndOfFile);
                    };

                    let Some(_) = bytes.take_prefix(64 - (length % 64)) else {
                        return Err(LoadError::UnexpectedEndOfFile);
                    };

                    let blob = IndexEntry {
                        state: ValidationState::Unvalidated,
                        bytes: blob_bytes,
                    };
                    index.insert(hash, Mutex::new(blob));
                }
                MAGIC_MARKER_BRANCH => {
                    let Ok(header) = bytes.view_prefix::<BranchHeader>() else {
                        return Err(LoadError::HeaderError);
                    };
                    let Some(branch_id) = Id::new(header.branch_id) else {
                        return Err(LoadError::HeaderError);
                    };
                    let hash = Value::new(header.hash);
                    branches.insert(branch_id, hash);
                }
                _ => return Err(LoadError::MagicMarkerError),
            };
        }

        let index = RwLock::new(index);
        let branches = RwLock::new(branches);

        let file = Mutex::new(AppendFile {
            file,
            length: file_len,
        });

        Ok(Self {
            file,
            mmap,
            index,
            branches,
        })
    }

    #[must_use]
    fn insert_blob_raw(
        &self,
        hash: Value<Hash<H>>,
        validation: ValidationState,
        bytes: &Bytes,
    ) -> Result<Bytes, InsertError> {
        let mut append = self.file.lock().unwrap();

        let old_length = append.length;
        let padding = 64 - (bytes.len() % 64);

        let new_length = old_length + 64 + bytes.len() + padding;
        if new_length > MAX_PILE_SIZE {
            return Err(InsertError::PileTooLarge);
        }

        append.length = new_length;

        let now_in_sys = SystemTime::now();
        let now_since_epoch = now_in_sys
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards");
        let now_in_ms = now_since_epoch.as_millis();

        let header = BlobHeader::new(now_in_ms as u64, bytes.len() as u64, hash);

        append.file.write_all(header.as_bytes())?;
        append.file.write_all(bytes)?;
        append.file.write_all(&[0; 64][0..padding])?;

        let written_bytes = unsafe {
            let written_slice =
                slice_from_raw_parts(self.mmap.as_ptr().offset(old_length as _), bytes.len())
                    .as_ref()
                    .unwrap();
            Bytes::from_raw_parts(written_slice, self.mmap.clone())
        };

        let mut index = self.index.write()?;
        index.insert(
            hash,
            Mutex::new(IndexEntry {
                state: validation,
                bytes: written_bytes.clone(),
            }),
        );

        Ok(written_bytes)
    }

    pub fn insert_blob<T: BlobSchema>(&self, blob: Blob<T>) -> Result<Value<Hash<H>>, InsertError> {
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
        let index = self.index.read().unwrap();
        let Some(blob) = index.get(hash) else {
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
        &self,
        branch_id: Id,
        old_hash: Option<Value<Hash<H>>>,
        new_hash: Value<Hash<H>>,
    ) -> Result<(), UpdateBranchError<H>> {
        let mut append = self.file.lock().unwrap();

        {
            let branches = self.branches.read().unwrap();
            let current_hash = branches.get(&branch_id);
            if current_hash != old_hash.as_ref() {
                return Err(UpdateBranchError::PreviousHashMismatch(
                    current_hash.cloned(),
                ));
            }
        }

        let new_length = append.length + 64;
        if new_length > MAX_PILE_SIZE {
            return Err(UpdateBranchError::PileTooLarge);
        }

        append.length = new_length;

        let header = BranchHeader::new(branch_id, new_hash);

        append.file.write_all(header.as_bytes())?;

        let mut branches = self.branches.write()?;
        branches.insert(branch_id, new_hash);

        Ok(())
    }

    pub fn set_branch(&self, branch_id: Id, hash: Value<Hash<H>>) -> Result<(), InsertError> {
        let mut append = self.file.lock().unwrap();

        let new_length = append.length + 64;
        if new_length > MAX_PILE_SIZE {
            return Err(InsertError::PileTooLarge);
        }

        append.length = new_length;

        let header = BranchHeader::new(branch_id, hash);

        append.file.write_all(header.as_bytes())?;

        let mut branches = self.branches.write()?;
        branches.insert(branch_id, hash);

        Ok(())
    }

    pub fn get_branch(&self, branch_id: Id) -> Option<Value<Hash<H>>> {
        let branches = self.branches.read().unwrap();
        branches.get(&branch_id).copied()
    }

    pub fn flush(&self) -> Result<(), FlushError> {
        let append = self.file.lock()?;
        append.file.sync_data()?;
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

use super::{ListBlobs, ListBranches, PullBlob, PullBranch, PushBlob, PushBranch, PushResult};

impl<const MAX_PILE_SIZE: usize, H> ListBlobs<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = std::convert::Infallible;

    fn list<'a>(
        &'a self,
    ) -> impl futures::stream::Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>> {
        let index = self.index.read().unwrap();
        let keys: Vec<Result<Value<Handle<H, UnknownBlob>>, _>> = index
            .keys()
            .copied()
            .map(|hash| Ok(hash.transmute()))
            .collect();
        futures::stream::iter(keys)
    }
}

impl<const MAX_PILE_SIZE: usize, H> PullBlob<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = GetBlobError;

    fn pull<T>(
        &self,
        hash: Value<crate::prelude::valueschemas::Handle<H, T>>,
    ) -> impl std::future::Future<Output = Result<Blob<T>, Self::Err>>
    where
        T: crate::prelude::BlobSchema + 'static,
    {
        async move { self.get_blob(&hash) }
    }
}

impl<const MAX_PILE_SIZE: usize, H> PushBlob<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = InsertError;

    fn push<T: BlobSchema>(
        &self,
        blob: Blob<T>,
    ) -> impl futures::Future<Output = Result<Value<Handle<H, T>>, <Self as PushBlob<H>>::Err>>
    where
        T: crate::prelude::BlobSchema + 'static,
    {
        async move {
            let hash = self.insert_blob(blob)?;
            Ok(hash.transmute())
        }
    }
}

impl<const MAX_PILE_SIZE: usize, H> ListBranches<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = std::convert::Infallible;

    fn list<'a>(&'a self) -> impl futures::stream::Stream<Item = Result<Id, Self::Err>> {
        let branches = self.branches.read().unwrap();
        let keys: Vec<Result<Id, _>> = branches.keys().copied().map(|id| Ok(id)).collect();
        futures::stream::iter(keys)
    }
}

impl<const MAX_PILE_SIZE: usize, H> PullBranch<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = std::convert::Infallible;

    fn pull(
        &self,
        id: Id,
    ) -> impl std::future::Future<Output = Result<Option<Value<Hash<H>>>, Self::Err>> {
        Box::pin(async move { Ok(self.get_branch(id)) })
    }
}

impl<const MAX_PILE_SIZE: usize, H> PushBranch<H> for Pile<MAX_PILE_SIZE, H>
where
    H: HashProtocol,
{
    type Err = UpdateBranchError<H>;

    fn push(
        &self,
        id: Id,
        old: Option<Value<Hash<H>>>,
        new: Value<Hash<H>>,
    ) -> impl std::future::Future<Output = Result<super::PushResult<H>, Self::Err>> {
        async move {
            let result = self.update_branch(id, old, new);
            match result {
                Ok(()) => Ok(PushResult::Success()),
                Err(UpdateBranchError::PreviousHashMismatch(old)) => Ok(PushResult::Conflict(old)),
                Err(err) => Err(err),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::RngCore;
    use tempfile;

    #[test]
    fn load() {
        const RECORD_LEN: usize = 1 << 10; // 1k
        const RECORD_COUNT: usize = 1 << 20; // 1M
        const MAX_PILE_SIZE: usize = 1 << 30; // 100GB

        let mut rng = rand::thread_rng();
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp_pile = tmp_dir.path().join("test.pile");
        let pile: Pile<MAX_PILE_SIZE> = Pile::load(&tmp_pile).unwrap();

        (0..RECORD_COUNT).for_each(|_| {
            let mut record = Vec::with_capacity(RECORD_LEN);
            rng.fill_bytes(&mut record);

            let data: Blob<UnknownBlob> = Blob::new(Bytes::from_source(record));
            pile.insert_blob(data).unwrap();
        });

        pile.flush().unwrap();

        drop(pile);

        let _pile: Pile<MAX_PILE_SIZE> = Pile::load(&tmp_pile).unwrap();
    }
}
