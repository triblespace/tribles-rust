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
//! On the surface deletion and forgetting seem to be the same thing.
//! However there is a subtle difference: When you delete something,
//! you say that it no longer exists. When you forget something, you
//! say that you no longer know about it. This subtle difference is
//! important when you are working with data that is shared between
//! multiple parties. When you delete something, you are saying that
//! it no longer exists for anyone, and deleting a statement or fact
//! implies that it is no longer true. When you forget something, you
//! are saying that you no longer know about it, but the statement or
//! fact is still generally valid. More importantly, when you forget
//! something, you are not saying that others should forget it as well,
//! nor does it imply that statements or facts derived from the forgotten
//! statement or fact are no longer valid.
//!
//! This property is called _monotonicity_, and it has a deep relationship
//! with _consistency_, as laid out by the [CALM theorem](https://arxiv.org/abs/1901.01930)
//! (Consistency as Logical Monotonicity). The CALM theorem states that a distributed
//! system is consistent if and only if it is logically monotonic. This means that
//! if you want to build a consistent distributed system, you need to
//! ensure that it is logically monotonic. This is where forgetting comes
//! in: By allowing you to forget things, but preventing you from deleting
//! things, `tribles` allows you to build consistent distributed systems.
//!

pub mod blobschemas;
pub mod valueschemas;

pub use crate::blob::BlobSchema;
pub use crate::blob::{Blob, BlobSet, FromBlob, ToBlob, TryFromBlob, TryToBlob};
pub use crate::id::{fucid, local_ids, rngid, ufoid, ExclusiveId, Id, IdOwner, RawId};
pub use crate::namespace::NS;
pub use crate::query::{
    find,
    intersectionconstraint::{and, IntersectionConstraint},
};
pub use crate::trible::{Trible, TribleSet};
pub use crate::value::{FromValue, ToValue, TryFromValue, TryToValue, Value, ValueSchema};
