use ed25519::Signature;
use ed25519_dalek::{SignatureError, SigningKey, Verifier, VerifyingKey};
use itertools::{ExactlyOneError, Itertools};

use ed25519::signature::Signer;

use crate::{
    blob::schemas::simplearchive::SimpleArchive,
    id::{ExclusiveId, Id},
    namespace::NS,
    query::find,
    trible::TribleSet,
    value::{
        schemas::{
            ed25519::{self as ed, ED25519RComponent, ED25519SComponent},
            hash::{Blake3, Handle},
            shortstring::ShortString,
        },
        ToValue, Value,
    },
};

NS! {
    /// The `commits` namespace contains attributes describing commits in a repository.
    /// Commits are a fundamental building block of version control systems.
    /// They represent a snapshot of the repository at a specific point in time.
    /// Commits are immutable, append-only, and form a chain of history.
    /// Each commit is identified by a unique hash, and contains a reference to the previous commit.
    /// Commits are signed by the author, and can be verified by anyone with the author's public key.
    pub namespace commits {
        /// The actual data of the commit.
        "4DD4DDD05CC31734B03ABB4E43188B1F" as tribles: Handle<Blake3, SimpleArchive>;
        /// A commit that this commit is based on.
        "317044B612C690000D798CA660ECFD2A" as parent: Handle<Blake3, SimpleArchive>;
        /// The author of the commit identified by their ed25519 public key.
        "ADB4FFAD247C886848161297EFF5A05B" as authored_by: ed::ED25519PublicKey;
        /// The `r` part of the ed25519 signature of the commit.
        "9DF34F84959928F93A3C40AEB6E9E499" as signature_r: ed::ED25519RComponent;
        /// The `s` part of the ed25519 signature of the commit.
        "1ACE03BF70242B289FDF00E4327C3BC6" as signature_s: ed::ED25519SComponent;
        /// A short message describing the commit.
        /// Used by tools displaying the commit history.
        "12290C0BE0E9207E324F24DDE0D89300" as short_message: ShortString;
    }
}

pub enum ValidationError {
    AmbiguousSignature,
    MissingSignature,
    FailedValidation,
}

impl<I> From<ExactlyOneError<I>> for ValidationError
where
    I: Iterator,
{
    fn from(err: ExactlyOneError<I>) -> Self {
        let (lower_bound, _) = err.size_hint();
        match lower_bound {
            0 => ValidationError::MissingSignature,
            _ => ValidationError::AmbiguousSignature,
        }
    }
}

impl From<SignatureError> for ValidationError {
    fn from(_: SignatureError) -> Self {
        ValidationError::FailedValidation
    }
}

pub fn sign(
    signing_key: SigningKey,
    handle: Value<Handle<Blake3, SimpleArchive>>,
    commit_id: ExclusiveId,
) -> Result<TribleSet, ValidationError> {
    let hash = handle.bytes;
    let signature = signing_key.sign(&hash);
    let r = ED25519RComponent::from_signature(signature);
    let s = ED25519SComponent::from_signature(signature);
    let tribles = commits::entity!(&commit_id,
    {
        tribles: handle,
        authored_by: signing_key.verifying_key(),
        signature_r: r,
        signature_s: s,
    });
    Ok(tribles)
}

pub fn verify(tribles: TribleSet, commit_id: Id) -> Result<(), ValidationError> {
    let (payload, verifying_key, r, s) = find!(
    (payload: Value<_>, key: Value<_>, r, s),
    commits::pattern!(&tribles, [
    {(commit_id) @
        tribles: payload,
        authored_by: key,
        signature_r: r,
        signature_s: s
    }]))
    .exactly_one()?;

    let hash = payload.bytes;
    let signature = Signature::from_components(r, s);
    let verifying_key: VerifyingKey = verifying_key.try_from_value()?;
    verifying_key.verify(&hash, &signature)?;
    Ok(())
}
