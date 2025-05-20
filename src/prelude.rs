//! This module re-exports the most commonly used types and traits from the `tribles` crate.
//! It is intended to be glob imported as `use tribles::prelude::*;`.
//!
//! # Introduction
//!
//! The `tribles` crate is a Rust library for working with graph data.
//! It is designed to be simple, fast, and flexible.
//!
//! # Deletion and Forgetting
//!
//! On the surface, deletion and forgetting may seem identical.
//! However, there is a subtle but crucial difference: Deletion removes
//! a statement from existence, making it no longer valid, whereas forgetting removes your knowledge
//! of it, without affecting its validity. This distinction is particularly
//! important in contexts where data is shared among multiple parties, or where
//! derived statements are based on the original data.
//! Forgetting does not propagate to other parties,
//! is reversible should the forgotten information be rediscovered,
//! and does not invalidate any derived statements or facts.
//!
//! The property that distinguishes forgetting from deletion is called _monotonicity_,
//! and it has a deep relationship with _consistency_, as laid out by the [CALM theorem](https://arxiv.org/abs/1901.01930)
//! (Consistency as Logical Monotonicity). The CALM theorem states that a distributed
//! system is consistent if and only if it is logically monotonic. This means that
//! if you want to build a consistent distributed system, you need to
//! ensure that it is logically monotonic. This is where forgetting comes
//! in: _By allowing you to forget things, but preventing you from deleting
//! things, `tribles` allows you to build consistent distributed systems.
//!

pub mod blobschemas;
pub mod valueschemas;

pub use crate::blob::BlobSchema;
pub use crate::blob::{Blob, FromBlob, MemoryBlobStore, ToBlob, TryFromBlob, TryToBlob};
pub use crate::id::{fucid, local_ids, rngid, ufoid, ExclusiveId, Id, IdOwner, RawId};
pub use crate::namespace::NS;
pub use crate::query::{
    find,
    intersectionconstraint::{and, IntersectionConstraint},
};
pub use crate::repo::pile::Pile;
pub use crate::trible::{Trible, TribleSet};
pub use crate::value::{FromValue, ToValue, TryFromValue, TryToValue, Value, ValueSchema};
