use crate::{
    blob::{Blob, BlobSchema, PackBlob, TryUnpackBlob},
    id::RawId,
    trible::{A_END, A_START, E_END, E_START},
    tribleset::TribleSet,
};

use anybytes::{Bytes, PackedSlice};
use hex_literal::hex;
use std::convert::TryInto;

pub struct SimpleArchive;

impl BlobSchema for SimpleArchive {
    const ID: RawId = hex!("8F4A27C8581DADCBA1ADA8BA228069B6");
}

impl PackBlob<SimpleArchive> for TribleSet {
    fn pack(&self) -> Blob<SimpleArchive> {
        let mut tribles: Vec<[u8; 64]> = Vec::with_capacity(self.len());
        tribles.extend(self.eav.iter_prefix::<64>().map(|p| p.0));
        let bytes: Bytes = tribles.into();
        Blob::new(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnarchiveError {
    BadArchive,
    BadTriple,
    BadCanonicalizationRedundancy,
    BadCanonicalizationOrdering,
}

impl TryUnpackBlob<'_, SimpleArchive> for TribleSet {
    type Error = UnarchiveError;

    fn try_unpack(blob: &Blob<SimpleArchive>) -> Result<Self, Self::Error> {
        let mut tribles = TribleSet::new();

        let mut prev_trible = None;
        let Ok(packed_tribles): Result<PackedSlice<[u8; 64]>, _> = (&blob.bytes).try_into() else {
            return Err(UnarchiveError::BadArchive);
        };
        for t in packed_tribles.iter() {
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
