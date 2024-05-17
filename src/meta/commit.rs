use ed25519::Signature;
use ed25519_dalek::SigningKey;
use itertools::Itertools;

use ed25519::signature::{Signer, Verifier};

use crate::{namespace::NS, query::find, triblearchive::SimpleArchive, Handle, Id, TribleSet, types::{ hash::Blake3, ed25519 as ed, SmallString, ed25519::{RComponent, SComponent}}};

NS! {
    pub namespace commit_ns {
        "4DD4DDD05CC31734B03ABB4E43188B1F" as tribles: Handle<Blake3, SimpleArchive>;
        "12290C0BE0E9207E324F24DDE0D89300" as short_message: SmallString;
        "ADB4FFAD247C886848161297EFF5A05B" as authored_by: Id;
        "9DF34F84959928F93A3C40AEB6E9E499" as ed25519_signature_r: ed::RComponent;
        "1ACE03BF70242B289FDF00E4327C3BC6" as ed25519_signature_s: ed::SComponent;
        "B57D92D4630F8F1B697DAF49CDFA3757" as ed25519_pubkey: ed::VerifyingKey;
    }
}

pub struct ValidationError {
    #[allow(unused)]
    msg: String,
}

impl ValidationError {
    pub fn new(msg: &str) -> ValidationError {
        ValidationError {
            msg: msg.to_owned(),
        }
    }
}

pub fn sign(
    signing_key: SigningKey,
    handle: Handle<Blake3, SimpleArchive>,
    commit_id: Id,
) -> Result<TribleSet, ValidationError> {
    let hash = handle.hash.bytes;
    let signature = signing_key.sign(&hash);
    let r = RComponent::from_signature(signature);
    let s = SComponent::from_signature(signature);
    let tribles = commit_ns::entity!(commit_id,
    {
        tribles: handle,
        ed25519_pubkey: signing_key.verifying_key(),
        ed25519_signature_r: r,
        ed25519_signature_s: s,
    });
    Ok(tribles)
}

pub fn verify(tribles: TribleSet, commit_id: Id) -> Result<(), ValidationError> {
    let (payload, verifying_key, r, s) = find!(
        ctx,
        (payload, key, r, s),
        commit_ns::pattern!(ctx, tribles, [
        {(commit_id) @
            tribles: payload,
            ed25519_pubkey: key,
            ed25519_signature_r: r,
            ed25519_signature_s: s
        }])
    )
    .at_most_one()
    .map_err(|_| ValidationError::new("ambiguous signature in commit"))?
    .ok_or(ValidationError::new("no signature in commit"))?
    .map_err(|_| ValidationError::new("unexpected bad value in tribles"))?;

    let hash = payload.hash.bytes;
    let signature = Signature::from_components(r.0, s.0);
    verifying_key
        .verify(&hash, &signature)
        .map_err(|_| ValidationError::new("couldn't validate signature"))
}
