//! This module re-exports the most commonly used types and traits from the `tribles` crate.
//! It is intended to be glob imported as `use tribles::prelude::*;`.
//!
//! # Introduction
//!
//! The `tribles` crate is a Rust library for working with graph data.
//! It is designed to be simple, fast, and flexible.

pub mod blobschemas;
pub mod valueschemas;

pub use crate::blob::BlobSchema;
pub use crate::blob::{Blob, BlobSet, FromBlob, ToBlob, TryFromBlob, TryToBlob};
pub use crate::id::{fucid, local_ids, rngid, ufoid, Id, IdOwner, OwnedId, RawId};
pub use crate::namespace::NS;
pub use crate::query::{
    find,
    intersectionconstraint::{and, IntersectionConstraint},
};
pub use crate::trible::{Trible, TribleSet};
pub use crate::value::{FromValue, ToValue, TryFromValue, TryToValue, Value, ValueSchema};
