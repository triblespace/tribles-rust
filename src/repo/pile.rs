//! A Pile is an append-only collection of blobs and branches stored in a single
//! file. It is designed as a durable local repository storage that can be safely
//! shared between threads.
//!
//! For layout and recovery details see the [Pile
//! Format](../../book/src/pile-format.md) chapter of the Tribles Book.

use anybytes::Bytes;
use fs4::fs_std::FileExt;
use hex_literal::hex;
use memmap2::MmapOptions;
use reft_light::Apply;
use reft_light::ReadHandle;
use reft_light::WriteHandle;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{IoSlice, Seek, SeekFrom, Write};
use std::ops::Bound;
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

enum PileOps<H: HashProtocol> {
    Insert(Value<Hash<H>>, Bytes),
    Load(Value<Hash<H>>, Bytes, u64),
}

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
struct IndexEntry {
    state: Arc<OnceLock<ValidationState>>,
    bytes: Bytes,
    timestamp: u64,
}

impl IndexEntry {
    fn new(bytes: Bytes, timestamp: u64, validation: Option<ValidationState>) -> Self {
        Self {
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
/// It tracks the current file handle and memory mapping while keeping the latest
/// branch heads observed in the pile.
pub(crate) struct PileAux<const MAX_PILE_SIZE: usize, H: HashProtocol> {
    file: File,
    mmap: Arc<memmap2::MmapRaw>,
    branches: HashMap<Id, Value<Handle<H, SimpleArchive>>>,
    applied_length: usize,
}

fn padding_for_blob(blob_size: usize) -> usize {
    (BLOB_ALIGNMENT - ((BLOB_HEADER_LEN + blob_size) % BLOB_ALIGNMENT)) % BLOB_ALIGNMENT
}

fn refresh_range<const MAX_PILE_SIZE: usize, H: HashProtocol>(
    aux: &mut PileAux<MAX_PILE_SIZE, H>,
    start: usize,
    end: usize,
) -> Vec<PileOps<H>> {
    let mut bytes = unsafe {
        let slice = slice_from_raw_parts(aux.mmap.as_ptr().add(start), end - start)
            .as_ref()
            .unwrap();
        Bytes::from_raw_parts(slice, aux.mmap.clone())
    };
    let mut ops = Vec::new();
    while !bytes.is_empty() {
        let magic = bytes[0..16].try_into().unwrap();
        match magic {
            MAGIC_MARKER_BLOB => {
                let header = bytes.view_prefix::<BlobHeader>().unwrap();
                let data_len = header.length as usize;
                let pad = (BLOB_ALIGNMENT - (data_len % BLOB_ALIGNMENT)) % BLOB_ALIGNMENT;
                let blob_bytes = bytes.take_prefix(data_len).unwrap();
                bytes.take_prefix(pad).unwrap();
                let hash = Value::new(header.hash);
                let ts = header.timestamp;
                ops.push(PileOps::Load(hash, blob_bytes, ts));
            }
            MAGIC_MARKER_BRANCH => {
                let header = bytes.view_prefix::<BranchHeader>().unwrap();
                let branch_id = Id::new(header.branch_id).unwrap();
                let hash = Value::new(header.hash);
                aux.branches.insert(branch_id, hash);
            }
            _ => break,
        }
    }
    aux.applied_length = end;
    ops
}

impl<const MAX_PILE_SIZE: usize, H: HashProtocol> Apply<PileSwap<H>, PileAux<MAX_PILE_SIZE, H>>
    for PileOps<H>
{
    fn apply_first(
        &mut self,
        first: &mut PileSwap<H>,
        _second: &PileSwap<H>,
        auxiliary: &mut PileAux<MAX_PILE_SIZE, H>,
    ) {
        match self {
            PileOps::Insert(hash, bytes) => {
                let old = auxiliary.applied_length;
                let padding = padding_for_blob(bytes.len());

                let now_in_sys = SystemTime::now();
                let now_since_epoch = now_in_sys
                    .duration_since(UNIX_EPOCH)
                    .expect("time went backwards");
                let now_in_ms = now_since_epoch.as_millis();

                let header = BlobHeader::new(now_in_ms as u64, bytes.len() as u64, *hash);
                let padding_buf = [0u8; BLOB_ALIGNMENT];
                let slices = [
                    IoSlice::new(header.as_bytes()),
                    IoSlice::new(bytes.as_ref()),
                    IoSlice::new(&padding_buf[..padding]),
                ];
                let expected = BLOB_HEADER_LEN + bytes.len() + padding;
                let written = auxiliary
                    .file
                    .write_vectored(&slices)
                    .expect("failed to write blob record");
                assert_eq!(written, expected, "failed to write blob record");

                let end = auxiliary
                    .file
                    .seek(SeekFrom::Current(0))
                    .expect("failed to get file position") as usize;
                assert_eq!(end % BLOB_ALIGNMENT, 0, "pile misaligned after blob write");
                let start = end - written;

                if start != old {
                    let ops = refresh_range(auxiliary, old, start);
                    for op in ops {
                        if let PileOps::Load(hash, bytes, ts) = op {
                            first.blobs.insert(hash, IndexEntry::new(bytes, ts, None));
                        }
                    }
                }

                let blob_start = start + BLOB_HEADER_LEN;
                let written_bytes = unsafe {
                    let written_slice =
                        slice_from_raw_parts(auxiliary.mmap.as_ptr().add(blob_start), bytes.len())
                            .as_ref()
                            .unwrap();
                    Bytes::from_raw_parts(written_slice, auxiliary.mmap.clone())
                };
                first.blobs.insert(
                    *hash,
                    IndexEntry {
                        state: Arc::new(OnceLock::from(ValidationState::Validated)),
                        bytes: written_bytes.clone(),
                        timestamp: now_in_ms as u64,
                    },
                );
                auxiliary.applied_length = end;
            }
            PileOps::Load(hash, bytes, timestamp) => {
                first
                    .blobs
                    .insert(*hash, IndexEntry::new(bytes.clone(), *timestamp, None));
            }
        }
    }

    fn apply_second(
        self,
        first: &PileSwap<H>,
        second: &mut PileSwap<H>,
        _auxiliary: &mut PileAux<MAX_PILE_SIZE, H>,
    ) {
        let hash = match self {
            PileOps::Insert(hash, _) => hash,
            PileOps::Load(hash, _, _) => hash,
        };
        let first = first.blobs.get(&hash).expect("handle must exist in first");
        second.blobs.entry(hash).or_insert_with(|| IndexEntry {
            state: first.state.clone(),
            bytes: first.bytes.clone(),
            timestamp: first.timestamp,
        });
    }
}

/// A grow-only collection of blobs and branch pointers backed by a single file on disk.
///
/// The pile acts as an append-only log where new blobs or branch updates are appended
/// while an in-memory index is kept for fast retrieval.
pub struct Pile<const MAX_PILE_SIZE: usize, H: HashProtocol = Blake3> {
    w_handle: WriteHandle<PileOps<H>, PileSwap<H>, PileAux<MAX_PILE_SIZE, H>>,
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

    /// Returns the metadata for the given blob handle if it exists.
    pub fn metadata<S>(&self, handle: Value<Handle<H, S>>) -> Option<BlobMetadata>
    where
        S: BlobSchema,
        Handle<H, S>: ValueSchema,
    {
        let hash: &Value<Hash<H>> = handle.as_transmute();
        let r_handle = self.r_handle.enter()?;
        let entry = r_handle.blobs.get(hash)?;
        Some(BlobMetadata {
            timestamp: entry.timestamp,
            length: entry.bytes.len() as u64,
        })
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
                    Ok(value) => Ok(value),
                    Err(e) => Err(GetBlobError::ConversionError(e)),
                }
            }
            ValidationState::Invalid => Err(GetBlobError::ValidationError(entry.bytes.clone())),
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
            OpenError::IoError(err) => write!(f, "IO error: {err}"),
            OpenError::PileTooLarge => write!(f, "Pile too large"),
            OpenError::CorruptPile { valid_length } => {
                write!(f, "Corrupt pile at byte {valid_length}")
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
            InsertError::IoError(err) => write!(f, "IO error: {err}"),
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
            UpdateBranchError::IoError(err) => write!(f, "IO error: {err}"),
            UpdateBranchError::PileTooLarge => write!(f, "Pile too large"),
        }
    }
}

impl std::fmt::Display for UpdateBranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateBranchError::IoError(err) => write!(f, "IO error: {err}"),
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
    pub fn open(path: &Path) -> Result<Self, OpenError> {
        match Self::try_open(path) {
            Ok(pile) => Ok(pile),
            Err(OpenError::CorruptPile { valid_length }) => {
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
    /// [`OpenError::CorruptPile`] with the length of the valid prefix so the
    /// caller may decide how to handle it.
    pub fn try_open(path: &Path) -> Result<Self, OpenError> {
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;
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

        while !bytes.is_empty() {
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
                    let pad = (BLOB_ALIGNMENT - (data_len % BLOB_ALIGNMENT)) % BLOB_ALIGNMENT;
                    let hash = Value::new(header.hash);
                    let blob_bytes = bytes.take_prefix(data_len).ok_or(OpenError::CorruptPile {
                        valid_length: start_offset,
                    })?;
                    bytes.take_prefix(pad).ok_or(OpenError::CorruptPile {
                        valid_length: start_offset,
                    })?;
                    let timestamp = header.timestamp;
                    blobs.insert(hash, IndexEntry::new(blob_bytes, timestamp, None));
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
                    file,
                    mmap,
                    branches,
                    applied_length: length,
                },
            ),
        })
    }

    /// Refreshes in-memory state from newly appended records.
    pub fn refresh(&mut self) -> Result<(), std::io::Error> {
        let ops = {
            let aux = self.w_handle.auxiliary_mut();
            let file_len = aux.file.metadata()?.len() as usize;
            if file_len <= aux.applied_length {
                Vec::new()
            } else {
                refresh_range(aux, aux.applied_length, file_len)
            }
        };
        if !ops.is_empty() {
            self.w_handle.extend(ops);
            self.w_handle.flush();
        }
        Ok(())
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

use super::BlobStore;
use super::BlobStoreGet;
use super::BlobStoreList;
use super::BlobStorePut;
use super::BranchStore;
use super::PushResult;

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
        Some(((*hash).into(), Blob::new(bytes)))
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

        self.refresh().map_err(InsertError::IoError)?;

        let aux = self.w_handle.auxiliary_mut();
        let blob_size = blob.bytes.len();
        let file_len = aux.file.metadata()?.len() as usize;
        let padding = padding_for_blob(blob_size);
        let new_length = file_len + BLOB_HEADER_LEN + blob_size + padding;
        if new_length > MAX_PILE_SIZE {
            return Err(InsertError::PileTooLarge);
        }

        let handle: Value<Handle<H, S>> = blob.get_handle();
        let hash = handle.into();

        let bytes = blob.bytes;
        self.w_handle.append(PileOps::Insert(hash, bytes));

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
        self.flush().map_err(|e| match e {
            FlushError::IoError(err) => UpdateBranchError::IoError(err),
        })?;
        self.refresh().map_err(UpdateBranchError::IoError)?;

        {
            let file = &self.w_handle.auxiliary().file;
            file.lock_exclusive()?;
        }

        self.refresh().map_err(UpdateBranchError::IoError)?;

        let result = {
            let aux = self.w_handle.auxiliary_mut();
            let current_hash = aux.branches.get(&id);
            if current_hash != old.as_ref() {
                FileExt::unlock(&aux.file)?;
                return Ok(PushResult::Conflict(current_hash.cloned()));
            }

            let header_len = std::mem::size_of::<BranchHeader>();
            let file_len = aux.file.metadata()?.len() as usize;
            let new_length = file_len + header_len;
            if new_length > MAX_PILE_SIZE {
                FileExt::unlock(&aux.file)?;
                return Err(UpdateBranchError::PileTooLarge);
            }

            aux.branches.insert(id, new);

            let header = BranchHeader::new(id, new);
            let expected = header_len;
            let written = match aux.file.write(header.as_bytes()) {
                Ok(n) => n,
                Err(e) => {
                    FileExt::unlock(&aux.file)?;
                    return Err(UpdateBranchError::IoError(e));
                }
            };
            if written != expected {
                FileExt::unlock(&aux.file)?;
                return Err(UpdateBranchError::IoError(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "failed to write branch header",
                )));
            }
            let end = aux.file.seek(SeekFrom::Current(0))? as usize;
            assert_eq!(
                end % BLOB_ALIGNMENT,
                0,
                "pile misaligned after branch write"
            );
            aux.applied_length = end;
            FileExt::unlock(&aux.file)?;
            Ok(PushResult::Success())
        };

        result
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
            Err(OpenError::CorruptPile { valid_length }) => {
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
            let reader = pile.reader();
            let fetched: Blob<UnknownBlob> = reader.get(handle).unwrap();
            assert_eq!(fetched.bytes.as_ref(), data.as_slice());
        }

        pile.flush().unwrap();
        drop(pile);

        let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).unwrap();
        let reader = pile.reader();
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

        let stored: Blob<UnknownBlob> = pile.reader().get(handle).unwrap();
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

    // recover_grow test removed as growth strategy no longer exists
}
