use ed25519::Signature;
use ed25519_dalek::{SignatureError, SigningKey, Verifier, VerifyingKey};
use itertools::Itertools;

use ed25519::signature::Signer;

use super::repo;

use crate::{
    blob::{schemas::simplearchive::SimpleArchive, Blob},
    prelude::valueschemas::Handle,
    query::find,
    trible::TribleSet,
    value::Value,
};

use crate::value::schemas::hash::Blake3;

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

pub fn commit(
    signing_key: &SigningKey,
    parents: impl IntoIterator<Item = Value<Handle<Blake3, SimpleArchive>>>,
    msg: Option<&str>,
    content: Option<Blob<SimpleArchive>>,
) -> TribleSet {
    let mut commit = TribleSet::new();
    let commit_entity = crate::id::rngid();

    if let Some(content) = content {
        let handle = content.get_handle();
        let signature = signing_key.sign(&content.bytes);

        commit += repo::entity!(&commit_entity,
        {
            content: handle,
            signed_by: signing_key.verifying_key(),
            signature_r: signature,
            signature_s: signature,
        });
    }

    if let Some(msg) = msg {
        commit += repo::entity!(&commit_entity,
        {
            short_message: msg,
        });
    }

    for parent in parents {
        commit += repo::entity!(&commit_entity,
        {
            parent: parent,
        });
    }

    commit
}

pub fn verify(content: Blob<SimpleArchive>, metadata: TribleSet) -> Result<(), ValidationError> {
    let handle = content.get_handle();
    let (pubkey, r, s) = match find!(
    (pubkey: Value<_>, r, s),
    repo::pattern!(&metadata, [
    {
        content: (handle),
        signed_by: pubkey,
        signature_r: r,
        signature_s: s
    }]))
    .at_most_one()
    {
        Ok(Some(result)) => result,
        Ok(None) => return Err(ValidationError::MissingSignature),
        Err(_) => return Err(ValidationError::AmbiguousSignature),
    };

    let pubkey: VerifyingKey = pubkey.try_from_value()?;
    let signature = Signature::from_components(r, s);
    pubkey.verify(&content.bytes, &signature)?;
    Ok(())
}
