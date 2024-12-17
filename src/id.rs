//! [RawId]/[OwnedId] are 128bit high-entropy persistent identifiers.
//!
//! # General Remarks on Identifiers
//! We found it useful to split identifiers into three categories:
//! - human readable names (e.g. attribute names, head labels, e.t.c)
//!   
//!   These should be used deliberately and be bounded by a context:
//!   attribute names for example are contextualized by a namespace;
//!   heads are bound to a system environment.
//! - persistent identifiers (e.g. UUIDs, UFOIDs, RNGIDs)
//!   
//!   These opaquely address an identity with associated content that is
//!   changing, extensible or yet undetermined.
//!   While this might seem similar to a human readable names,
//!   it is important that they do not have any semantic meaning
//!   or cultural connotation.
//!   They must be cheaply generatable/mintable without any coordination,
//!   while being globally valid and unique.[^1]
//! - content addressed identifiers (e.g. hashes or signatures)
//!   
//!   Are essentially a unique fingerprint of the identified information.
//!   They not only allow two seperate entities to agree on the same identifier
//!   without any coordination, but also allow for easy content validation.
//!   They are the identifier of choice for any kind of immutable data (e.g. triblesets, blobs),
//!   and should be combined with the other identifiers even in mutable cases.
//!
//! To give an example from the world of scientific publishing.
//! Ideally published paper artefacts (e.g. .html and .pdf files) would
//! be identified and _referenced/cited_ by their hash.
//! Each published artifact/version should contain a persistent identifier
//! allowing the different versions to be tied together as one logical paper.
//! The abbreviations that a paper uses for its citations and bibliography
//! are then an example of a human readable name scoped to that paper.
//!
//! # Consistency, Directed Edges and Owned IDs
//!
//! While a simple grow set already constitutes a CRDT, it is also limited in
//! expressiveness. To provide richer semantics while guaranteeing conflict-free
//! mergeability we allow only "owned" IDs to be used in the `entity` position
//! of newly generated triples. As owned IDs are [send] but not [sync] owning a
//! set of them essentially constitutes a single writer transaction domain,
//! allowing for some non-monotonic operations like `if-does-not-exist`, over
//! the set of contained entities. Note that this does not make operations that
//! would break CALM safe, e.g. `delete`.
//!
//! A different perspective is that edges are always ordered from describing
//! to described entities, with circles constituting consensus between entities.
//!
//!
//!
//! # High-entropy persistent identifiers
//!
//! The only approach to generate persistent identifiers in a distributed setting
//! (i.e. without coordinating authority), is to give them enough entropy that
//! they
//! A valid concern one might have is
//! They have no predictability and are extremely resistant to
//! random bit errors. However this comes at the cost of no locality and
//! compressability.
//!
//!
//! |                | rngid | ufoid | fucid |
//! |----------------|-------|-------|-------|
//! | entropy        | high  | high  | low   |
//! | locality       | none  | high  | high  |
//! | compression    | none  | low   | high  |
//! | predictability | none  | low   | mid   |
//!
//! ## RNGID / Random Number Generator ID
//! Are generated by simply taking 128bits from a cryptographic random
//! source. They are easy to implement and provide the maximum possible amount
//! of entropy at the cost of locality and compressability. However UFOIDs are
//! almost universally a better choice, unless the use-case is incompatible with
//! leaking the time at which an id was minted.
//!
//!
//! ## UFOID / Universal Forgettable Ordered IDs
//! Are generated by concatenaing a rolling 32bit millisecond timestamp with
//! 96bits of cryptographic randomness. This provides high locality for ufoids
//! minted in close temporal proximity, and it provides fast and simple
//! identification of "old" ids in contexts where high data generation requires
//! regular garbage collection of stale information (e.g. robotics).
//! A range of 2^32 milliseconds corresponds to a rolling window of ~50 days,
//! which should be wide enough for most high-volume use cases, while still
//! providing sufficient bits of high-quality entropy. Having a practically
//! relevant rollover rate, also ensures that this edge case is accounted for
//! and tested, compared to schemes which place the burden of dealing with
//! overflows on future generations.
//!
//!
//! ## FUCID / Fast Unsafe Compressible IDs
//! Are generated by XORing a 128bit `salt` with a 128bit incrementing integer.
//! The salt is unique for each source generating fucids (i.e. each thread),
//! and initialized with 128bits of cryptographic randomness.
//! This creates a pseudo-random exhaustive walk through the 128bit space,
//! while maintaining high per-source locality.
//! It might seem counter intuitive that an identifier with low entropy
//! can still be used in a distributed setting. Note however fucids only have
//! low entropy between each other when they are from the same source (thread).
//! This makes them succeptible to collisions due to hardware errors (bit-flips),
//! but they still retain high global uniqueness without coordination.
//! E.g. in a scenario where every source only generates a single fucid the
//! scheme degenerates to the rngid scheme.
//!
//!
//! [^1]: An reader familiar with digital/academic publishing might recognize
//! that DOIs do not fit these criteria. In fact they combine the worst of
//! both worlds as non-readable human-readable names with semantic information
//! about the minting organisation.
//!

pub mod fucid;
pub mod rngid;
pub mod ufoid;

use std::{
    borrow::Borrow, cell::RefCell, convert::TryInto, hash::Hash, marker::PhantomData, mem,
    num::NonZero, ops::Deref,
};

pub use fucid::fucid;
pub use rngid::rngid;
pub use ufoid::ufoid;

use crate::{
    patch::{Entry, IdentityOrder, SingleSegmentation, PATCH},
    prelude::valueschemas::GenId,
    query::{Constraint, ContainsConstraint, Variable},
    value::{RawValue, VALUE_LEN},
};

thread_local!(static OWNED_IDS: RefCell<IdOwner> = RefCell::new(IdOwner::new()));

pub const ID_LEN: usize = 16;
pub type RawId = [u8; ID_LEN];

pub(crate) fn id_into_value(id: &RawId) -> RawValue {
    let mut data = [0; VALUE_LEN];
    data[16..32].copy_from_slice(id);
    data
}

pub(crate) fn id_from_value(id: &RawValue) -> Option<RawId> {
    if id[0..16] != [0; 16] {
        return None;
    }
    let id = id[16..32].try_into().unwrap();
    Some(id)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct Id {
    inner: NonZero<u128>,
}

impl Id {
    pub const fn new(id: RawId) -> Option<Self> {
        unsafe { std::mem::transmute(id) }
    }

    pub fn transmute_raw(id: &RawId) -> Option<&Self> {
        if *id == [0; 16] {
            None
        } else {
            Some(unsafe { std::mem::transmute(id) })
        }
    }

    /// Takes ownership of this Id from the current write context (thread).
    /// Returns `None` if this Id was not found, because it is not associated with this
    /// write context, or because it is currently aquired.
    pub fn aquire(&self) -> Option<OwnedId> {
        OWNED_IDS.with_borrow_mut(|owner| owner.take(self))
    }
}

impl PartialOrd for Id {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let s: &RawId = &self;
        let o: &RawId = &other;
        PartialOrd::partial_cmp(s, o)
    }
}

impl Ord for Id {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let s: &RawId = &self;
        let o: &RawId = &other;
        Ord::cmp(s, o)
    }
}

impl Hash for Id {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let s: &RawId = &self;
        Hash::hash(s, state);
    }
}

impl Deref for Id {
    type Target = RawId;

    fn deref(&self) -> &Self::Target {
        unsafe { std::mem::transmute(self) }
    }
}

impl Borrow<RawId> for Id {
    fn borrow(&self) -> &RawId {
        self
    }
}

impl AsRef<[u8; 16]> for Id {
    fn as_ref(&self) -> &[u8; 16] {
        self
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        &self[..]
    }
}

impl From<Id> for RawId {
    fn from(id: Id) -> Self {
        *id
    }
}

impl From<Id> for RawValue {
    fn from(id: Id) -> Self {
        id.into()
    }
}

#[macro_export]
macro_rules! id_hex {
    ( $data:expr ) => {
        $crate::id::Id::new(hex_literal::hex!($data)).unwrap()
    };
}

pub use id_hex;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct OwnedId {
    pub id: Id,
    // Make sure that the type can't be syntactically initialized.
    // Also make sure that we we don't get auto impl of Send and Sync
    _private: PhantomData<*const ()>,
}

unsafe impl Send for OwnedId {}

impl OwnedId {
    pub fn force(id: Id) -> Self {
        Self {
            id,
            _private: PhantomData,
        }
    }

    pub fn transmute_force<'a>(id: &'a Id) -> &'a Self {
        unsafe { std::mem::transmute(id) }
    }

    pub fn release(self) -> Id {
        let id = self.id;
        mem::drop(self);
        id
    }

    pub fn forget(self) -> Id {
        let id = self.id;
        mem::forget(self);
        id
    }
}

impl Drop for OwnedId {
    fn drop(&mut self) {
        OWNED_IDS.with_borrow_mut(|ids| {
            ids.force_insert(self);
        });
    }
}

impl Deref for OwnedId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl Borrow<RawId> for OwnedId {
    fn borrow(&self) -> &RawId {
        self
    }
}

impl Borrow<Id> for OwnedId {
    fn borrow(&self) -> &Id {
        self
    }
}

impl AsRef<[u8; 16]> for OwnedId {
    fn as_ref(&self) -> &[u8; 16] {
        self
    }
}

impl AsRef<[u8]> for OwnedId {
    fn as_ref(&self) -> &[u8] {
        &self[..]
    }
}

impl From<OwnedId> for RawId {
    fn from(value: OwnedId) -> Self {
        **value
    }
}

pub fn local_owned(v: Variable<GenId>) -> impl Constraint<'static> {
    OWNED_IDS.with_borrow(|owner| owner.has(v))
}

pub struct IdOwner {
    owned_ids: PATCH<ID_LEN, IdentityOrder, SingleSegmentation>,
}

impl IdOwner {
    pub fn new() -> Self {
        Self {
            owned_ids: PATCH::new(),
        }
    }

    pub fn insert(&mut self, owned_id: OwnedId) -> Id {
        self.force_insert(&owned_id);
        owned_id.forget()
    }

    pub fn force_insert(&mut self, id: &Id) {
        let entry = Entry::new(&id);
        self.owned_ids.insert(&entry);
    }

    pub fn take(&mut self, id: &Id) -> Option<OwnedId> {
        if self.owned_ids.has_prefix(id) {
            self.owned_ids.remove(id);
            Some(OwnedId::force(*id))
        } else {
            None
        }
    }

    pub fn owns(&mut self, id: &Id) -> bool {
        self.owned_ids.has_prefix(id)
    }
}

impl ContainsConstraint<'static, GenId> for &IdOwner {
    type Constraint = <PATCH<ID_LEN, IdentityOrder, SingleSegmentation> as ContainsConstraint<
        'static,
        GenId,
    >>::Constraint;

    fn has(self, v: Variable<GenId>) -> Self::Constraint {
        self.owned_ids.clone().has(v)
    }
}

#[cfg(test)]
mod tests {
    use crate::id::OwnedId;
    use crate::prelude::*;
    use crate::tests::literature;

    #[test]
    fn ns_local_owned() {
        let mut kb = TribleSet::new();

        {
            let romeo = ufoid();
            let juliet = ufoid();
            kb += literature::entity!(&juliet, {
                firstname: "Jules",
                lastname: "Verne"
            });
            kb += literature::entity!(&romeo, {
                firstname: "Isaac",
                lastname: "Asimov"
            });
        }

        let mut r: Vec<_> = find!(
            (author: OwnedId, name: String),
            and!(
                local_owned(author),
                literature::pattern!(&kb, [
                    {author @
                        firstname: name
                    }])
            )
        )
        .map(|(_, n)| n)
        .collect();
        r.sort();

        assert_eq!(vec!["Isaac", "Jules"], r);
    }
}
