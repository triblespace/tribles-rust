//! A Pile is an append-only collection of blobs and branches stored in a single
//! file. It is designed as a durable local repository storage that can be safely
//! shared between threads.
//!
//! For layout and recovery details see the [Pile
//! Format](../../book/src/pile-format.md) chapter of the Tribles Book.

use anybytes::Bytes;
use hex_literal::hex;
use memmap2::MmapOptions;
use memmap2::MmapRaw;
use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::IoSlice;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;
use std::ptr::slice_from_raw_parts;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;
use zerocopy::TryFromBytes;

use crate::blob::schemas::UnknownBlob;
use crate::blob::Blob;
use crate::blob::BlobSchema;
use crate::blob::ToBlob;
use crate::blob::TryFromBlob;
use crate::id::Id;
use crate::id::RawId;
use crate::patch::Entry;
use crate::patch::IdentitySchema;
use crate::patch::PATCHIterator;
use crate::patch::PATCH;
use crate::prelude::blobschemas::SimpleArchive;
use crate::prelude::valueschemas::Handle;
use crate::value::schemas::hash::Blake3;
use crate::value::schemas::hash::Hash;
use crate::value::schemas::hash::HashProtocol;
use crate::value::RawValue;
use crate::value::Value;
use crate::value::ValueSchema;

const MAGIC_MARKER_BLOB: RawId = hex!("1E08B022FF2F47B6EBACF1D68EB35D96");
const MAGIC_MARKER_BRANCH: RawId = hex!("2BC991A7F5D5D2A3A468C53B0AA03504");

const BLOB_HEADER_LEN: usize = std::mem::size_of::<BlobHeader>();
const BLOB_ALIGNMENT: usize = BLOB_HEADER_LEN;

#[derive(Debug, Clone, Copy)]
pub enum ValidationState {
    Validated,
    Invalid,
}

#[derive(Debug, Clone, Copy)]
pub struct BlobMetadata {
    pub timestamp: u64,
    pub length: u64,
}

#[derive(Debug, Clone)]
enum IndexEntry {
    InFlight {
        len: u64,
        timestamp: u64,
    },
    Stored {
        state: Arc<OnceLock<ValidationState>>,
        bytes: Bytes,
        timestamp: u64,
    },
}

impl IndexEntry {
    fn in_flight(len: u64, timestamp: u64) -> Self {
        Self::InFlight { len, timestamp }
    }

    fn stored(bytes: Bytes, timestamp: u64, validation: Option<ValidationState>) -> Self {
        Self::Stored {
            state: Arc::new(validation.map(OnceLock::from).unwrap_or_default()),
            bytes,
            timestamp,
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

#[derive(Debug)]
/// A grow-only collection of blobs and branch pointers backed by a single file on disk.
pub struct Pile<const MAX_PILE_SIZE: usize, H: HashProtocol = Blake3> {
    file: File,
    mmap: Arc<MmapRaw>,
    blobs: PATCH<32, IdentitySchema, IndexEntry>,
    branches: HashMap<Id, Value<Handle<H, SimpleArchive>>>,
    applied_length: usize,
}

fn padding_for_blob(blob_size: usize) -> usize {
    (BLOB_ALIGNMENT - ((BLOB_HEADER_LEN + blob_size) % BLOB_ALIGNMENT)) % BLOB_ALIGNMENT
}

#[derive(Debug, Clone)]
/// Read-only handle referencing a [`Pile`].
///
/// Multiple `PileReader` instances can coexist and provide concurrent access to
/// the same underlying pile data.
pub struct PileReader<H: HashProtocol> {
    blobs: PATCH<32, IdentitySchema, IndexEntry>,
    _marker: std::marker::PhantomData<H>,
}

impl<H: HashProtocol> PartialEq for PileReader<H> {
    fn eq(&self, other: &Self) -> bool {
        self.blobs == other.blobs
    }
}

impl<H: HashProtocol> Eq for PileReader<H> {}

impl<H: HashProtocol> PileReader<H> {
    fn new(blobs: PATCH<32, IdentitySchema, IndexEntry>) -> Self {
        Self {
            blobs,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns an iterator over all blobs currently stored in the pile.
    pub fn iter(&self) -> PileBlobStoreIter<'_, H> {
        PileBlobStoreIter {
            patch: &self.blobs,
            inner: self.blobs.iter(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the metadata for the given blob handle if it exists.
    pub fn metadata<S>(&self, handle: Value<Handle<H, S>>) -> Option<BlobMetadata>
    where
        S: BlobSchema,
        Handle<H, S>: ValueSchema,
    {
        let hash: &Value<Hash<H>> = handle.as_transmute();
        let entry = self.blobs.get(&hash.raw)?;
        match entry {
            IndexEntry::Stored {
                timestamp, bytes, ..
            } => Some(BlobMetadata {
                timestamp: *timestamp,
                length: bytes.len() as u64,
            }),
            IndexEntry::InFlight { timestamp, len } => Some(BlobMetadata {
                timestamp: *timestamp,
                length: *len,
            }),
        }
    }
}

impl<H: HashProtocol> BlobStoreGet<H> for PileReader<H> {
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
        let Some(entry) = self.blobs.get(&hash.raw) else {
            return Err(GetBlobError::BlobNotFound);
        };
        match entry {
            IndexEntry::Stored { state, bytes, .. } => {
                let state = state.get_or_init(|| {
                    let computed_hash = Hash::<H>::digest(bytes);
                    if computed_hash == *hash {
                        ValidationState::Validated
                    } else {
                        ValidationState::Invalid
                    }
                });
                match state {
                    ValidationState::Validated => {
                        let blob: Blob<S> = Blob::new(bytes.clone());
                        match blob.try_from_blob() {
                            Ok(value) => Ok(value),
                            Err(e) => Err(GetBlobError::ConversionError(e)),
                        }
                    }
                    ValidationState::Invalid => Err(GetBlobError::ValidationError(bytes.clone())),
                }
            }
            IndexEntry::InFlight { .. } => Err(GetBlobError::BlobNotFound),
        }
    }
}

impl<H: HashProtocol, const MAX_PILE_SIZE: usize> BlobStore<H> for Pile<MAX_PILE_SIZE, H> {
    type Reader = PileReader<H>;
    type ReaderError = ReadError;

    fn reader(&mut self) -> Result<Self::Reader, Self::ReaderError> {
        self.refresh()?;
        Ok(PileReader::new(self.blobs.clone()))
    }
}

#[derive(Debug)]
pub enum ReadError {
    IoError(std::io::Error),
    PileTooLarge,
    CorruptPile { valid_length: usize },
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::IoError(err) => write!(f, "IO error: {err}"),
            ReadError::PileTooLarge => write!(f, "Pile too large"),
            ReadError::CorruptPile { valid_length } => {
                write!(f, "Corrupt pile at byte {valid_length}")
            }
        }
    }
}
impl std::error::Error for ReadError {}

impl From<std::io::Error> for ReadError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<ReadError> for std::io::Error {
    fn from(err: ReadError) -> Self {
        match err {
            ReadError::IoError(e) => e,
            ReadError::PileTooLarge => {
                std::io::Error::new(std::io::ErrorKind::Other, "pile too large")
            }
            ReadError::CorruptPile { valid_length } => std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("corrupt pile at byte {valid_length}"),
            ),
        }
    }
}

#[derive(Debug)]
pub enum InsertError {
    IoError(std::io::Error),
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::IoError(err) => write!(f, "IO error: {err}"),
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
}

impl std::error::Error for UpdateBranchError {}

unsafe impl Send for UpdateBranchError {}
unsafe impl Sync for UpdateBranchError {}

impl std::fmt::Debug for UpdateBranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateBranchError::IoError(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl std::fmt::Display for UpdateBranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateBranchError::IoError(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl From<std::io::Error> for UpdateBranchError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<ReadError> for UpdateBranchError {
    fn from(err: ReadError) -> Self {
        Self::IoError(err.into())
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
            GetBlobError::ConversionError(err) => write!(f, "Conversion error: {err}"),
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
    pub fn open(path: &Path) -> Result<Self, ReadError> {
        match Self::try_open(path) {
            Ok(pile) => Ok(pile),
            Err(ReadError::CorruptPile { valid_length }) => {
                // Truncate the file at the first valid offset and try again.
                OpenOptions::new()
                    .write(true)
                    .open(path)?
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
    /// [`ReadError::CorruptPile`] with the length of the valid prefix so the
    /// caller may decide how to handle it.
    pub fn try_open(path: &Path) -> Result<Self, ReadError> {
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;
        let length = file.metadata()?.len() as usize;
        if length > MAX_PILE_SIZE {
            return Err(ReadError::PileTooLarge);
        }

        let mmap = MmapOptions::new()
            .len(MAX_PILE_SIZE)
            .map_raw_read_only(&file)?;
        let mmap = Arc::new(mmap);

        let mut pile = Self {
            file,
            mmap,
            blobs: PATCH::<32, IdentitySchema, IndexEntry>::new(),
            branches: HashMap::new(),
            applied_length: 0,
        };

        pile.refresh()?;
        Ok(pile)
    }

    /// Refreshes in-memory state from newly appended records.
    pub fn refresh(&mut self) -> Result<(), ReadError> {
        let file_len = self.file.metadata()?.len() as usize;
        if file_len > MAX_PILE_SIZE {
            return Err(ReadError::PileTooLarge);
        }
        if file_len > self.applied_length {
            let start = self.applied_length;
            let mut bytes = unsafe {
                let slice = slice_from_raw_parts(self.mmap.as_ptr().add(start), file_len - start)
                    .as_ref()
                    .unwrap();
                Bytes::from_raw_parts(slice, self.mmap.clone())
            };
            while !bytes.is_empty() {
                let start_offset = file_len - bytes.len();
                if bytes.len() < 16 {
                    return Err(ReadError::CorruptPile {
                        valid_length: start_offset,
                    });
                }
                let magic = bytes[0..16].try_into().unwrap();
                match magic {
                    MAGIC_MARKER_BLOB => {
                        let header = bytes.view_prefix::<BlobHeader>().map_err(|_| {
                            ReadError::CorruptPile {
                                valid_length: start_offset,
                            }
                        })?;
                        let data_len = header.length as usize;
                        let pad = (BLOB_ALIGNMENT - (data_len % BLOB_ALIGNMENT)) % BLOB_ALIGNMENT;
                        let blob_bytes =
                            bytes.take_prefix(data_len).ok_or(ReadError::CorruptPile {
                                valid_length: start_offset,
                            })?;
                        bytes.take_prefix(pad).ok_or(ReadError::CorruptPile {
                            valid_length: start_offset,
                        })?;
                        let hash: Value<Hash<H>> = Value::new(header.hash);
                        let ts = header.timestamp;
                        let entry =
                            Entry::with_value(&hash.raw, IndexEntry::stored(blob_bytes, ts, None));
                        if !matches!(self.blobs.get(&hash.raw), Some(IndexEntry::Stored { .. })) {
                            self.blobs.replace(&entry);
                        }
                    }
                    MAGIC_MARKER_BRANCH => {
                        let header = bytes.view_prefix::<BranchHeader>().map_err(|_| {
                            ReadError::CorruptPile {
                                valid_length: start_offset,
                            }
                        })?;
                        let branch_id =
                            Id::new(header.branch_id).ok_or(ReadError::CorruptPile {
                                valid_length: start_offset,
                            })?;
                        let hash: Value<Hash<H>> = Value::new(header.hash);
                        self.branches.insert(branch_id, hash.into());
                    }
                    _ => {
                        return Err(ReadError::CorruptPile {
                            valid_length: start_offset,
                        })
                    }
                }
            }
            self.applied_length = file_len;
        }
        Ok(())
    }

    /// Persists all writes to the underlying pile file.
    pub fn flush(&mut self) -> Result<(), FlushError> {
        self.file.sync_data()?;
        Ok(())
    }
}

impl<const MAX_PILE_SIZE: usize, H: HashProtocol> Drop for Pile<MAX_PILE_SIZE, H> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

use super::BlobStore;
use super::BlobStoreGet;
use super::BlobStoreList;
use super::BlobStorePut;
use super::BranchStore;
use super::PushResult;

/// Iterator returned by [`PileReader::iter`].
///
/// Iterates over all `(Handle, Blob)` pairs currently stored in the pile.
pub struct PileBlobStoreIter<'a, H: HashProtocol> {
    patch: &'a PATCH<32, IdentitySchema, IndexEntry>,
    inner: PATCHIterator<'a, 32, IdentitySchema, IndexEntry>,
    _marker: std::marker::PhantomData<H>,
}

impl<'a, H: HashProtocol> Iterator for PileBlobStoreIter<'a, H> {
    type Item = (Value<Handle<H, UnknownBlob>>, Blob<UnknownBlob>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(key) = self.inner.next() {
            let entry = self.patch.get(key)?;
            if let IndexEntry::Stored { bytes, .. } = entry {
                let hash: Value<Hash<H>> = Value::new(*key);
                return Some((hash.into(), Blob::new(bytes.clone())));
            }
        }
        None
    }
}

/// Adapter over [`PileBlobStoreIter`] that yields only the blob handles.
pub struct PileBlobStoreListIter<'a, H: HashProtocol> {
    inner: PileBlobStoreIter<'a, H>,
}

impl<'a, H: HashProtocol> Iterator for PileBlobStoreListIter<'a, H> {
    type Item = Result<Value<Handle<H, UnknownBlob>>, Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        let (handle, _) = self.inner.next()?;
        Some(Ok(handle))
    }
}

impl<H: HashProtocol> BlobStoreList<H> for PileReader<H> {
    type Err = Infallible;
    type Iter<'a> = PileBlobStoreListIter<'a, H>;

    fn blobs(&self) -> Self::Iter<'_> {
        PileBlobStoreListIter { inner: self.iter() }
    }
}

impl<const MAX_PILE_SIZE: usize, H: HashProtocol> BlobStorePut<H> for Pile<MAX_PILE_SIZE, H> {
    type PutError = InsertError;

    fn put<S, T>(&mut self, item: T) -> Result<Value<Handle<H, S>>, Self::PutError>
    where
        S: BlobSchema + 'static,
        T: ToBlob<S>,
        Handle<H, S>: ValueSchema,
    {
        let blob = ToBlob::to_blob(item);

        let blob_size = blob.bytes.len();
        let padding = padding_for_blob(blob_size);

        let handle: Value<Handle<H, S>> = blob.get_handle();
        let hash: Value<Hash<H>> = handle.into();

        if self.blobs.get(&hash.raw).is_some() {
            return Ok(handle.transmute());
        }

        let now_in_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis();
        let header = BlobHeader::new(now_in_ms as u64, blob_size as u64, hash);
        let expected = BLOB_HEADER_LEN + blob_size + padding;
        let padding_buf = [0u8; BLOB_ALIGNMENT];
        let bufs = [
            IoSlice::new(header.as_bytes()),
            IoSlice::new(blob.bytes.as_ref()),
            IoSlice::new(&padding_buf[..padding]),
        ];
        let written = self.file.write_vectored(&bufs)?;
        if written != expected {
            return Err(InsertError::IoError(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "failed to write blob record",
            )));
        }

        let entry = Entry::with_value(
            &hash.raw,
            IndexEntry::in_flight(blob_size as u64, now_in_ms as u64),
        );
        self.blobs.insert(&entry);

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
            iter: self.branches.keys(),
        }
    }

    fn head(&self, id: Id) -> Result<Option<Value<Handle<H, SimpleArchive>>>, Self::HeadError> {
        Ok(self.branches.get(&id).copied())
    }

    fn update(
        &mut self,
        id: Id,
        old: Option<Value<Handle<H, SimpleArchive>>>,
        new: Value<Handle<H, SimpleArchive>>,
    ) -> Result<super::PushResult<H>, Self::UpdateError> {
        self.flush().map_err(|e| match e {
            FlushError::IoError(err) => UpdateBranchError::IoError(err),
        })?;
        self.refresh().map_err(UpdateBranchError::from)?;

        {
            self.file.lock()?;
        }

        self.refresh().map_err(UpdateBranchError::from)?;

        let result = {
            let current_hash = self.branches.get(&id);
            if current_hash != old.as_ref() {
                self.file.unlock()?;
                return Ok(PushResult::Conflict(current_hash.cloned()));
            }

            let header_len = std::mem::size_of::<BranchHeader>();

            self.branches.insert(id, new);

            let header = BranchHeader::new(id, new);
            let expected = header_len;
            let written = match self.file.write(header.as_bytes()) {
                Ok(n) => n,
                Err(e) => {
                    self.file.unlock()?;
                    return Err(UpdateBranchError::IoError(e));
                }
            };
            if written != expected {
                self.file.unlock()?;
                return Err(UpdateBranchError::IoError(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "failed to write branch header",
                )));
            }
            let end = self.file.seek(SeekFrom::Current(0))? as usize;
            assert_eq!(
                end % BLOB_ALIGNMENT,
                0,
                "pile misaligned after branch write"
            );
            self.applied_length = end;
            if let Err(e) = self.file.sync_data() {
                self.file.unlock()?;
                return Err(UpdateBranchError::IoError(e));
            }
            self.file.unlock()?;
            Ok(PushResult::Success())
        };

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::repo::PushResult;
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
            Err(ReadError::CorruptPile { valid_length }) => assert_eq!(valid_length, 0),
            other => panic!("unexpected result: {other:?}"),
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
            Err(ReadError::CorruptPile { valid_length }) => {
                assert_eq!(valid_length as u64, file_len)
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn put_and_get_preserves_blob_bytes() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let data = vec![42u8; 100];
        let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(data.clone()));
        let handle = pile.put(blob).unwrap();

        {
            let reader = pile.reader().unwrap();
            let fetched: Blob<UnknownBlob> = reader.get(handle).unwrap();
            assert_eq!(fetched.bytes.as_ref(), data.as_slice());
        }

        pile.flush().unwrap();
        drop(pile);

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let reader = pile.reader().unwrap();
        let fetched: Blob<UnknownBlob> = reader.get(handle).unwrap();
        assert_eq!(fetched.bytes.as_ref(), data.as_slice());
    }

    #[test]
    fn blob_after_branch_is_clean() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();

        let branch_id = Id::new([1; 16]).unwrap();
        let head = Value::<Handle<Blake3, SimpleArchive>>::new([2; 32]);
        pile.update(branch_id, None, head).unwrap();

        let data = vec![3u8; 8];
        let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(data.clone()));
        let handle = pile.put(blob).unwrap();
        pile.flush().unwrap();

        let stored: Blob<UnknownBlob> = pile.reader().unwrap().get(handle).unwrap();
        assert_eq!(stored.bytes.as_ref(), &data[..]);
    }

    #[test]
    fn insert_after_branch_preserves_head() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let blob1: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 5]));
        let handle1 = pile.put(blob1).unwrap();

        let branch_id = Id::new([1u8; 16]).unwrap();
        pile.update(branch_id, None, handle1.transmute()).unwrap();

        let blob2: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![2u8; 5]));
        pile.put(blob2).unwrap();
        pile.flush().unwrap();
        drop(pile);

        let pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let head = pile.head(branch_id).unwrap();
        assert_eq!(head, Some(handle1.transmute()));
    }

    #[test]
    fn branch_update_without_flush_keeps_head() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let branch_id = Id::new([1u8; 16]).unwrap();

        let handle = {
            let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
            let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![3u8; 5]));
            let handle = pile.put(blob).unwrap();
            pile.update(branch_id, None, handle.transmute()).unwrap();
            std::mem::forget(pile);
            handle
        };

        let pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        assert_eq!(pile.head(branch_id).unwrap(), Some(handle.transmute()));
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }

    #[test]
    fn metadata_returns_length_and_timestamp() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![7u8; 32]));
        let handle = pile.put(blob).unwrap();
        pile.flush().unwrap();
        drop(pile);

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let reader = pile.reader().unwrap();
        let meta = reader.metadata(handle).unwrap();
        assert_eq!(meta.length, 32);
        assert!(meta.timestamp > 0);
    }

    #[test]
    fn iter_lists_all_blobs() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let blob1: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 4]));
        let h1 = pile.put(blob1).unwrap();
        let blob2: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![2u8; 4]));
        let h2 = pile.put(blob2).unwrap();
        pile.flush().unwrap();

        let reader = pile.reader().unwrap();
        let handles: Vec<_> = reader.iter().map(|(h, _)| h).collect();
        assert!(handles.contains(&h1));
        assert!(handles.contains(&h2));
        assert_eq!(handles.len(), 2);
    }

    #[test]
    fn update_conflict_returns_current_head() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let blob1: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 5]));
        let h1 = pile.put(blob1).unwrap();
        let branch_id = Id::new([1u8; 16]).unwrap();
        pile.update(branch_id, None, h1.transmute()).unwrap();
        pile.flush().unwrap();

        let blob2: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![2u8; 5]));
        let h2 = pile.put(blob2).unwrap();
        pile.flush().unwrap();

        match pile.update(branch_id, Some(h2.transmute()), h1.transmute()) {
            Ok(PushResult::Conflict(existing)) => {
                assert_eq!(existing, Some(h1.transmute()))
            }
            other => panic!("unexpected result: {other:?}"),
        }
        assert_eq!(pile.head(branch_id).unwrap(), Some(h1.transmute()));
    }

    #[test]
    fn refresh_errors_on_malformed_append() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 4]));
        pile.put(blob).unwrap();
        pile.flush().unwrap();

        use std::io::Write;
        {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            file.write_all(b"garbage").unwrap();
            file.sync_data().unwrap();
        }

        assert!(pile.refresh().is_err());
    }

    #[test]
    fn put_duplicate_blob_does_not_grow_file() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let data = vec![9u8; 32];
        let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(data.clone()));
        let handle1 = pile.put(blob).unwrap();
        pile.flush().unwrap();
        let len_after_first = std::fs::metadata(&path).unwrap().len();

        let blob_dup: Blob<UnknownBlob> = Blob::new(Bytes::from_source(data));
        let handle2 = pile.put(blob_dup).unwrap();
        pile.flush().unwrap();
        let len_after_second = std::fs::metadata(&path).unwrap().len();

        assert_eq!(handle1, handle2);
        assert_eq!(len_after_first, len_after_second);
    }

    #[test]
    fn branch_update_conflict_returns_existing_head() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let blob1: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![1u8; 8]));
        let blob2: Blob<UnknownBlob> = Blob::new(Bytes::from_source(vec![2u8; 8]));
        let h1 = pile.put(blob1).unwrap();
        let h2 = pile.put(blob2).unwrap();
        pile.flush().unwrap();

        let branch_id = Id::new([3u8; 16]).unwrap();
        pile.update(branch_id, None, h1.transmute()).unwrap();

        match pile.update(branch_id, Some(h2.transmute()), h2.transmute()) {
            Ok(PushResult::Conflict(existing)) => {
                assert_eq!(existing, Some(h1.transmute()))
            }
            other => panic!("expected conflict, got {other:?}"),
        }
        assert_eq!(pile.head(branch_id).unwrap(), Some(h1.transmute()));
    }

    #[test]
    fn metadata_reports_blob_length() {
        const MAX_PILE_SIZE: usize = 1 << 20;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pile.pile");

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let data = vec![7u8; 16];
        let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(data.clone()));
        let handle = pile.put(blob).unwrap();
        pile.flush().unwrap();

        let reader = pile.reader().unwrap();
        let meta = reader.metadata(handle).expect("metadata");
        assert_eq!(meta.length, data.len() as u64);
    }

    // recover_grow test removed as growth strategy no longer exists
}
