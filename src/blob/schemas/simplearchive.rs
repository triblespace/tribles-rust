use crate::{
    blob::{Blob, BlobSchema, ToBlob, TryFromBlob},
    id::Id,
    id_hex,
    trible::{Trible, TribleSet},
};

use anybytes::{Bytes, View};

pub struct SimpleArchive;

impl BlobSchema for SimpleArchive {
    const BLOB_SCHEMA_ID: Id = id_hex!("8F4A27C8581DADCBA1ADA8BA228069B6");
}

impl ToBlob<SimpleArchive> for &TribleSet {
    fn to_blob(self) -> Blob<SimpleArchive> {
        let mut tribles: Vec<[u8; 64]> = Vec::with_capacity(self.len());
        tribles.extend(self.eav.iter_ordered());
        let bytes: Bytes = tribles.into();
        Blob::new(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnarchiveError {
    BadArchive,
    BadTrible,
    BadCanonicalizationRedundancy,
    BadCanonicalizationOrdering,
}

impl TryFromBlob<SimpleArchive> for TribleSet {
    type Error = UnarchiveError;

    fn try_from_blob(blob: &Blob<SimpleArchive>) -> Result<Self, Self::Error> {
        let mut tribles = TribleSet::new();

        let mut prev_trible = None;
        let Ok(packed_tribles): Result<View<[[u8; 64]]>, _> = blob.bytes.clone().view() else {
            return Err(UnarchiveError::BadArchive);
        };
        for t in packed_tribles.iter() {
            if let Some(trible) = Trible::as_transmute_force_raw(t) {
                if let Some(prev) = prev_trible {
                    if prev == t {
                        return Err(UnarchiveError::BadCanonicalizationRedundancy);
                    }
                    if prev > t {
                        return Err(UnarchiveError::BadCanonicalizationOrdering);
                    }
                }
                prev_trible = Some(t);
                tribles.insert(trible);
            } else {
                return Err(UnarchiveError::BadTrible);
            }
        }

        Ok(tribles)
    }
}
