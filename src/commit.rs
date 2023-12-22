use std::convert::TryInto;

use blake2::{digest::typenum::U32, Blake2b, Digest};
use signature::Signer;
use itertools::Itertools;

use crate::trible::{A_END, A_START, E_END, E_START};
use crate::types::syntactic::RawId;
use crate::types::{BlobParseError, Bloblike, Idlike};
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
        signature_method: "6EFD5433E03E0231E25DE00D7E5A2244" as crate::types::syntactic::RawId;
        short_message: "12290C0BE0E9207E324F24DDE0D89300" as crate::types::syntactic::ShortString;
        authored_by: "ADB4FFAD247C886848161297EFF5A05B" as crate::types::syntactic::RawId;
        signature_ed25519_r: "9DF34F84959928F93A3C40AEB6E9E499" as crate::types::semantic::ed25519::RComponent;
        signature_ed25519_s: "1ACE03BF70242B289FDF00E4327C3BC6" as crate::types::semantic::ed25519::SComponent;
        signature_ed25519_pubkey: "B57D92D4630F8F1B697DAF49CDFA3757" as crate::types::semantic::ed25519::VerifyingKey;
    }
}

const BLAKE2B: Id = hex_literal::hex!("6F8AC972ABADFE295809DA070263EA05");

#[derive(Debug, Clone)]
pub struct Commit {
    id: Id,
    tribles: TribleSet,
}

pub struct ValidationError {
    msg: String
}

impl ValidationError {
    pub fn new(msg: &str) -> ValidationError {
        ValidationError {
            msg: msg.to_owned()
        }
    }
}

pub fn commit(tribles: TribleSet) -> Result<TribleSet, ValidationError>
    where
        T: Idlike,
    {
        let id = id.into_id();

        let (RawId(signature_method),) = query!(
            ctx,
            (f),
            commit_ns::pattern!(ctx, tribles, [
            {(RawId(id)) @
                signature_method: f
            }])
        )
        .at_most_one()
        .map_err(|e| ValidationError::new("ambiguous signature method found in commit tribles"))?
        .ok_or(ValidationError::new("no signature method found in commit tribles"))?
        .map_err(|e| ValidationError::new("invalid value in result"))?;

        if signature_method != BLAKE2B {
            return Err(ValidationError::new("unsupported checksum method"))
        }

        Ok(Self { id, tribles })
    }
}