//! [RawId]/[FreshId] are 128bit high-entropy persistent identifiers.
//!
//! # General Remarks on Identifiers
//! We found it useful to split identifiers into three categories:
//! - human readable names (e.g. attribute names, head labels, e.t.c)
//!   These should be used deliberately and be bounded by a context:
//!   attribute names for example are contextualized by a namespace;
//!   heads are bound to a system environment.
//! - persistent identifiers (e.g. UUIDs, UFOIDs, RNGIDs)
//!   These opaquely address an identity with associated content that is
//!   changing, extensible or yet undetermined.
//!   While this might seem similar to a human readable names,
//!   it is important that they do not have any semantic meaning
//!   or cultural connotation.
//!   They must be cheaply generatable/mintable without any coordination,
//!   while being globally valid and unique.[^1]
//! - content addressed identifiers (e.g. hashes or signatures)
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
//! # Consistency, Directed Edges and Fresh IDs
//!
//! While a simple grow set already constitutes a CRDT,
//! it is also limited in expressiveness.
//! To provide richer semantics while guaranteeing conflict-free
//! mergeability we allow (or at least strongly suggest) only "fresh"
//! IDs to be used in the `entity` position of newly generated triples.
//! As fresh IDs are [send] but not [sync] owning a set of them essentially
//! constitutes a single writer transaction domain, allowing for some non-monotonic
//! operations like `if-does-not-exist`, over the set of contained entities.
//! Note that this does not enable operations that would break CALM, e.g. `delete`.
//!
//! A different perspective is that edges are always ordered from describing
//! to described entities, with circles constituting consensus between entities.
//!
//! [^1]: An reader familiar with digital/academic publishing might recognize
//! that DOIs do not fit these criteria. In fact they combine the worst of
//! both worlds as non-readable human-readable names with semantic information
//! about the minting organisation.
//!

pub mod fucid;
pub mod rngid;
pub mod ufoid;

use std::convert::TryInto;

pub use fucid::fucid;
pub use rngid::rngid;
pub use ufoid::ufoid;

use crate::value::{RawValue, VALUE_LEN};

pub const ID_LEN: usize = 16;
pub type RawId = [u8; ID_LEN];

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FreshId {
    pub raw: RawId,
    _private: (),
}

impl FreshId {
    pub unsafe fn new(id: RawId) -> Self {
        FreshId {
            raw: id,
            _private: (),
        }
    }
}

unsafe impl Send for FreshId {}

impl From<FreshId> for RawId {
    fn from(value: FreshId) -> Self {
        value.raw
    }
}

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
