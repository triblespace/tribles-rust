use anybytes::Bytes;
pub use blake3::Hasher as Blake3;
use digest::Digest;
use hex_literal::hex;
use memmap2::MmapOptions;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::ptr::slice_from_raw_parts;
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, io::Write};
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::id::RawId;

pub type Hash = [u8; 32];

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
    hash: Hash,
}

impl BranchHeader {
    fn new(branch_id: RawId, hash: Hash) -> Self {
        Self {
            magic_marker: MAGIC_MARKER_BRANCH,
            branch_id,
            hash,
        }
    }
}

#[derive(TryFromBytes, IntoBytes, Immutable, KnownLayout, Copy, Clone)]
#[repr(C)]
struct BlobHeader {
    magic_marker: RawId,
    timestamp: u64,
    length: u64,
    hash: Hash,
}

impl BlobHeader {
    fn new(timestamp: u64, length: u64, hash: Hash) -> Self {
        Self {
            magic_marker: MAGIC_MARKER_BLOB,
            timestamp,
            length,
            hash,
        }
    }
}

pub struct Pile<const MAX_PILE_SIZE: usize> {
    file: Mutex<AppendFile>,
    mmap: Arc<memmap2::MmapRaw>,
    index: RwLock<HashMap<Hash, Mutex<IndexEntry>>>,
    branches: RwLock<HashMap<RawId, Hash>>,
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
pub enum GetError {
    PoisonError,
    ValidationError(Bytes),
}

impl<T> From<PoisonError<T>> for GetError {
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
impl<const MAX_PILE_SIZE: usize> Pile<MAX_PILE_SIZE> {
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
                    let hash = header.hash;
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
                    let branch_id = header.branch_id;
                    let hash = header.hash;
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
        &mut self,
        hash: Hash,
        validation: ValidationState,
        value: &Bytes,
    ) -> Result<Bytes, InsertError> {
        let mut append = self.file.lock().unwrap();

        let old_length = append.length;
        let padding = 64 - (value.len() % 64);

        let new_length = old_length + 64 + value.len() + padding;
        if new_length > MAX_PILE_SIZE {
            return Err(InsertError::PileTooLarge);
        }

        append.length = new_length;

        let now_in_sys = SystemTime::now();
        let now_since_epoch = now_in_sys
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards");
        let now_in_ms = now_since_epoch.as_millis();

        let header = BlobHeader::new(now_in_ms as u64, value.len() as u64, hash);

        append.file.write_all(header.as_bytes())?;
        append.file.write_all(&value)?;
        append.file.write_all(&[0; 64][0..padding])?;

        let written_bytes = unsafe {
            let written_slice =
                slice_from_raw_parts(self.mmap.as_ptr().offset(old_length as _), value.len())
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

    pub fn insert_blob(&mut self, value: &Bytes) -> Result<Hash, InsertError> {
        let hash: Hash = Blake3::digest(&value).into();

        let _bytes = self.insert_blob_raw(hash, ValidationState::Validated, value)?;

        Ok(hash)
    }

    pub fn insert_blob_validated(
        &mut self,
        hash: Hash,
        value: &Bytes,
    ) -> Result<Bytes, InsertError> {
        self.insert_blob_raw(hash, ValidationState::Validated, value)
    }

    pub fn insert_blob_unvalidated(
        &mut self,
        hash: Hash,
        value: &Bytes,
    ) -> Result<Bytes, InsertError> {
        self.insert_blob_raw(hash, ValidationState::Unvalidated, value)
    }

    pub fn get_blob(&self, hash: &Hash) -> Result<Option<Bytes>, GetError> {
        let index = self.index.read().unwrap();
        let Some(blob) = index.get(hash) else {
            return Ok(None);
        };
        let mut entry = blob.lock().unwrap();
        match entry.state {
            ValidationState::Validated => {
                return Ok(Some(entry.bytes.clone()));
            }
            ValidationState::Invalid => {
                return Err(GetError::ValidationError(entry.bytes.clone()));
            }
            ValidationState::Unvalidated => {
                let computed_hash: Hash = Blake3::digest(&entry.bytes).into();
                if computed_hash != *hash {
                    entry.state = ValidationState::Invalid;
                    return Err(GetError::ValidationError(entry.bytes.clone()));
                } else {
                    entry.state = ValidationState::Validated;
                    return Ok(Some(entry.bytes.clone()));
                }
            }
        }
    }

    pub fn commit_branch(&mut self, branch_id: RawId, hash: Hash) -> Result<(), InsertError> {
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

    pub fn get_branch(&self, branch_id: RawId) -> Option<Hash> {
        let branches = self.branches.read().unwrap();
        branches.get(&branch_id).copied()
    }

    pub fn flush(&self) -> Result<(), FlushError> {
        let append = self.file.lock()?;
        append.file.sync_data()?;
        Ok(())
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
        let mut pile: Pile<MAX_PILE_SIZE> = Pile::load(&tmp_pile).unwrap();

        (0..RECORD_COUNT).for_each(|_| {
            let mut record = Vec::with_capacity(RECORD_LEN);
            rng.fill_bytes(&mut record);

            let data = Bytes::from_source(record);
            pile.insert_blob(&data).unwrap();
        });

        pile.flush().unwrap();

        drop(pile);

        let _pile: Pile<MAX_PILE_SIZE> = Pile::load(&tmp_pile).unwrap();
    }
}
