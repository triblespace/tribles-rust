use ed25519::Signature;
use ed25519_dalek::SignatureError;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;
use itertools::Itertools;

use ed25519::signature::Signer;

use super::repo;

use crate::blob::schemas::simplearchive::SimpleArchive;
use crate::blob::Blob;
use crate::prelude::valueschemas::Handle;
use crate::query::find;
use crate::trible::TribleSet;
use crate::value::Value;

use crate::value::schemas::hash::Blake3;
use hifitime::Epoch;

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

/// Constructs commit metadata describing `content` and its parent commits.
///
/// The resulting [`TribleSet`] is signed using `signing_key` so that its
/// authenticity can later be verified. If `msg` is provided it is stored as a
/// short commit message.
pub fn commit(
    signing_key: &SigningKey,
    parents: impl IntoIterator<Item = Value<Handle<Blake3, SimpleArchive>>>,
    msg: Option<&str>,
    content: Option<Blob<SimpleArchive>>,
) -> TribleSet {
    let mut commit = TribleSet::new();
    let commit_entity = crate::id::rngid();
    let now = Epoch::now().expect("system time");

    commit += repo::entity!(&commit_entity, { timestamp: (now, now) });

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

/// Validates that the `metadata` blob genuinely signs the supplied commit
/// `content`.
///
/// Returns an error if the signature information is missing, malformed or does
/// not match the commit bytes.
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
