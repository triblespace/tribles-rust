use blake2::{digest::typenum::U32, Blake2b, Digest};
use itertools::Itertools;

use crate::{trible::{Id, Blob, TRIBLE_LEN, ID_LEN, Value}, tribleset::TribleSet, namespace::NS, query};
use crate::types::syntactic::RawId;

NS! {
    pub namespace commit_ns {
        @ crate::types::syntactic::RawId;
        fingerprint_method: "6EFD5433E03E0231E25DE00D7E5A2244" as crate::types::syntactic::RawId;
        short_message: "12290C0BE0E9207E324F24DDE0D89300" as crate::types::syntactic::ShortString;
        authored_by: "ADB4FFAD247C886848161297EFF5A05B" as crate::types::syntactic::RawId;
    }
}

const BLAKE2: Id = hex_literal::hex!("6F8AC972ABADFE295809DA070263EA05");

#[derive(Debug, Clone)]
pub struct Commit {
    id: Id,
    tribles: TribleSet
}

impl Commit {
    pub fn new<T>(id: T, tribles: TribleSet) -> Self
    where T: Into<Id> {
        let id = id.into();
        Commit {
            id,
            tribles
        }
    }

    pub fn serialize(&self) -> Blob {
        let mut buffer = Vec::<u8>::with_capacity((self.tribles.len() + 1)*64);

        let mut tribles = self.tribles.eav.infixes(&[0; TRIBLE_LEN], 0, TRIBLE_LEN, |k| k);
        tribles.sort_unstable();
        for trible in tribles {
            buffer.extend_from_slice(&trible);
        }

        buffer.extend_from_slice(&self.id);

        let (RawId(fingerprint_method),) = query!(
            ctx,
            (f),
            commit_ns::pattern!(ctx, self.tribles, [
                {(RawId(self.id)) @
                    fingerprint_method: f
                }])
        ).at_most_one()
        .expect("ambiguous fingerprint method found in commit tribles")
        .expect("no fingerprint method found in commit tribles");

        let fingerprint:Value = match fingerprint_method {
            BLAKE2 => {
                Blake2b::<U32>::digest(&buffer).into()
            }
            _ => panic!("unsupported fingerprint method")
        };
        
        buffer.extend_from_slice(&[0u8; ID_LEN]);
        buffer.extend_from_slice(&fingerprint);

        buffer.into()
    }
}


