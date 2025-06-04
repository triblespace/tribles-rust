use ed25519::{signature::Signer, Signature};
use ed25519_dalek::Verifier;
use ed25519_dalek::{SignatureError, SigningKey, VerifyingKey};
use itertools::Itertools;

use crate::{
    blob::Blob,
    find,
    id::{rngid, Id},
    metadata::metadata,
    prelude::blobschemas::SimpleArchive,
    trible::TribleSet,
    value::Value,
};

use super::repo;

pub fn branch(
    signing_key: &SigningKey,
    branch_id: Id,
    name: &str,
    commit_head: Blob<SimpleArchive>,
) -> TribleSet {
    let handle = commit_head.get_handle();
    let signature = signing_key.sign(&commit_head.bytes);

    let metadata_entity = rngid();

    let mut metadata: TribleSet = Default::default();

    metadata += repo::entity!(&metadata_entity,
    {
        branch: branch_id,
        head: handle,
        signed_by: signing_key.verifying_key(),
        signature_r: signature,
        signature_s: signature,
    });

    metadata += metadata::entity!(&metadata_entity,
    {
        name: name,
    });

    metadata
}

pub fn branch_unsigned(branch_id: Id, name: &str, commit_head: Blob<SimpleArchive>) -> TribleSet {
    let handle = commit_head.get_handle();

    let metadata_entity = rngid();

    let mut metadata: TribleSet = Default::default();

    metadata += repo::entity!(&metadata_entity,
    {
        branch: branch_id,
        head: handle,
    });

    metadata += metadata::entity!(&metadata_entity,
    {
        name: name,
    });

    metadata
}

pub enum ValidationError {
    AmbiguousSignature,
    MissingSignature,
    FailedValidation,
}

impl From<SignatureError> for ValidationError {
    fn from(_: SignatureError) -> Self {
        ValidationError::FailedValidation
    }
}

pub fn verify(
    commit_head: Blob<SimpleArchive>,
    metadata: TribleSet,
) -> Result<(), ValidationError> {
    let handle = commit_head.get_handle();
    let (pubkey, r, s) = match find!(
    (pubkey: Value<_>, r, s),
    repo::pattern!(&metadata, [
    {
        head: (handle),
        signed_by: pubkey,
        signature_r: r,
        signature_s: s,
    }]))
    .at_most_one()
    {
        Ok(Some(result)) => result,
        Ok(None) => return Err(ValidationError::MissingSignature),
        Err(_) => return Err(ValidationError::AmbiguousSignature),
    };

    let Ok(pubkey): Result<VerifyingKey, _> = pubkey.try_from_value() else {
        return Err(ValidationError::FailedValidation);
    };
    let signature = Signature::from_components(r, s);
    pubkey.verify(&commit_head.bytes, &signature)?;

    Ok(())
}
