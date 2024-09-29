use anybytes::Bytes;
use std::convert::TryInto;

use crate::{
    blobschemas::{PackBlob, TryUnpackBlob},
    trible::{A_END, A_START, E_END, E_START, TRIBLE_LEN},
    Blob, BlobSchema, TribleSet,
};

pub struct SimpleArchive;

impl BlobSchema for SimpleArchive {}

impl PackBlob<SimpleArchive> for TribleSet {
    fn pack(&self) -> crate::Blob<SimpleArchive> {
        let mut tribles: Vec<[u8; 64]> = Vec::with_capacity(self.len());
        tribles.extend(self.eav.iter_prefix::<64>().map(|p| p.0));
        let buffer: Vec<u8> = bytemuck::allocation::cast_vec(tribles);
        let bytes: Bytes = buffer.into();
        Blob::new(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnarchiveError {
    BadArchiveLength,
    BadTriple,
    BadCanonicalizationRedundancy,
    BadCanonicalizationOrdering,
}

impl TryUnpackBlob<'_, SimpleArchive> for TribleSet {
    type Error = UnarchiveError;

    fn try_unpack(blob: &Blob<SimpleArchive>) -> Result<Self, Self::Error> {
        let len: usize = blob.bytes.len();

        if len % TRIBLE_LEN != 0 {
            return Err(UnarchiveError::BadArchiveLength);
        }

        let mut tribles = TribleSet::new();

        let mut prev_trible = None;
        for trible in blob.bytes.chunks_exact(TRIBLE_LEN) {
            let t: &[u8; 64] = trible.try_into().unwrap();
            if t[E_START..=E_END] == [0; 16] {
                return Err(UnarchiveError::BadTriple);
            }
            if t[A_START..=A_END] == [0; 16] {
                return Err(UnarchiveError::BadTriple);
            }
            if let Some(prev) = prev_trible {
                if prev == t {
                    return Err(UnarchiveError::BadCanonicalizationRedundancy);
                }
                if prev > t {
                    return Err(UnarchiveError::BadCanonicalizationOrdering);
                }
            }
            prev_trible = Some(t);
            tribles.insert_raw(t);
        }

        Ok(tribles)
    }
}
