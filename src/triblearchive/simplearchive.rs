use std::convert::TryInto;

use bytes::Bytes;

use crate::{trible::{A_END, A_START, E_END, E_START, TRIBLE_LEN}, types::Hash, BlobParseError, Bloblike, Handle, TribleSet};

pub struct SimpleArchive(Bytes);

impl Bloblike for SimpleArchive {
    fn read_blob(blob: Bytes) -> Result<Self, BlobParseError> {
        let len: usize = blob.len();

        if len % TRIBLE_LEN != 0 {
            return Err(BlobParseError::new(
                "simple archive must be multiples of 64bytes long",
            ));
        }
        
        let mut prev_trible = None;
        for trible in blob.chunks_exact(TRIBLE_LEN) {
            let t: &[u8; 64] = trible.try_into().unwrap();
            if t[E_START..=E_END] == [0; 16] {
                return Err(BlobParseError::new(
                    "validation error: trible contains NULL id in E position",
                ));
            }
            if t[A_START..=A_END] == [0; 16] {
                return Err(BlobParseError::new(
                    "validation error: trible contains NULL id in A position",
                ));
            }
            if let Some(prev) = prev_trible {
                if prev == t {
                    return Err(BlobParseError::new(
                        "validation error: redundant trible",
                    ));
                }
                if prev > t {
                    return Err(BlobParseError::new(
                        "validation error: tribles must be sorted in ascending order",
                    ));
                }

                prev_trible = Some(t);
            }
        }

        Ok(SimpleArchive(blob))
    }

    fn into_blob(self) -> Bytes {
        self.0
    }

    fn as_handle<H>(&self) -> Handle<H, Self>
    where
        H: digest::Digest + digest::OutputSizeUser<OutputSize = digest::consts::U32>,
    {
        let digest = H::digest(&self.0);
        unsafe { Handle::new(Hash::new(digest.into())) }
    }
}

impl From<TribleSet> for SimpleArchive {
    fn from(value: TribleSet) -> Self {
        let mut tribles: Vec<[u8; 64]> = Vec::with_capacity(value.len());
        tribles.extend(value.eav.iter_prefix::<64>().map(|p| p.0));
        let buffer: Vec<u8> = bytemuck::allocation::cast_vec(tribles);
        SimpleArchive(buffer.into())
    }
}

impl From<SimpleArchive> for TribleSet {
    fn from(value: SimpleArchive) -> Self {
        let mut tribles = TribleSet::new();
        for t in value.0.chunks_exact(TRIBLE_LEN) {
            tribles.insert_raw(t.try_into().unwrap());
        }
        tribles
    }
}
