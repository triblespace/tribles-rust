use std::convert::TryInto;

use blake2::{digest::typenum::U32, Blake2b, Digest};
use itertools::Itertools;

use crate::types::syntactic::RawId;
use crate::{
    namespace::NS,
    query,
    trible::TRIBLE_LEN,
    tribleset::TribleSet,
    types::{Blob, Id, Value, ID_LEN},
};

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
    tribles: TribleSet,
}

impl Commit {
    pub fn new<T>(id: T, tribles: TribleSet) -> Self
    where
        T: Into<Id>,
    {
        let id = id.into();
        Self { id, tribles }
    }

    pub fn deserialize(data: &[u8]) -> Self {
        let len = data.len();

        assert!(
            len % TRIBLE_LEN == 0,
            "commits must be multiples of 64bytes long"
        );
        assert!(len > TRIBLE_LEN, "commits must not be empty");
        assert!(
            data[len - 48..len - 32] == [0; 16],
            "capstone marker missing"
        );

        let id: Id = data[len - 64..len - 48].try_into().unwrap();
        let stored_fingerprint: Value = data[len - 32..len].try_into().unwrap();
        let trible_data: &[u8] = &data[0..len - TRIBLE_LEN];
        let fingerprinted_data: &[u8] = &data[..len - 48];

        let mut tribles = TribleSet::new();

        for trible in trible_data.chunks_exact(TRIBLE_LEN) {
            tribles.insert_raw(trible.try_into().unwrap());
        }

        let (RawId(fingerprint_method),) = query!(
            ctx,
            (f),
            commit_ns::pattern!(ctx, tribles, [
            {(RawId(id)) @
                fingerprint_method: f
            }])
        )
        .at_most_one()
        .expect("ambiguous fingerprint method found in commit tribles")
        .expect("no fingerprint method found in commit tribles");

        let computed_fingerprint: Value = match fingerprint_method {
            BLAKE2 => Blake2b::<U32>::digest(fingerprinted_data).into(),
            _ => panic!("unsupported fingerprint method"),
        };

        assert!(
            stored_fingerprint == computed_fingerprint,
            "commit fingerprint doesn't match computed fingerprint"
        );

        Self { id, tribles }
    }

    pub fn serialize(&self) -> Blob {
        let mut buffer = Vec::<u8>::with_capacity((self.tribles.len() + 1) * 64);

        let mut tribles = self
            .tribles
            .eav
            .infixes(&[0; TRIBLE_LEN], 0, TRIBLE_LEN, |k| k);
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
        )
        .at_most_one()
        .expect("ambiguous fingerprint method found in commit tribles")
        .expect("no fingerprint method found in commit tribles");

        let fingerprint: Value = match fingerprint_method {
            BLAKE2 => Blake2b::<U32>::digest(&buffer).into(),
            _ => panic!("unsupported fingerprint method"),
        };

        buffer.extend_from_slice(&[0u8; ID_LEN]);
        buffer.extend_from_slice(&fingerprint);

        buffer.into()
    }
}
