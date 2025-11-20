//! Metadata namespace for the `triblespace` crate.
//!
//! This namespace is used to bootstrap the meaning of other namespaces.
//! It defines meta attributes that are used to describe other attributes.

use crate::blob::MemoryBlobStore;
use crate::id::Id;
use crate::id_hex;
use crate::prelude::valueschemas;
use crate::trible::TribleSet;
use crate::value::schemas::hash::Blake3;
use crate::value::ValueSchema;
use core::marker::PhantomData;
use triblespace_core_macros::attributes;

/// Describes metadata that can be emitted for documentation or discovery.
pub trait Metadata {
    /// Returns the root identifier for this metadata description.
    fn metadata_id(&self) -> Id;

    fn describe(&self) -> (TribleSet, MemoryBlobStore<Blake3>);
}

/// Helper trait for schema types that want to expose metadata without requiring an instance.
pub trait ConstMetadata: ValueSchema {
    /// Returns the root identifier for this metadata description.
    ///
    /// By default this mirrors the value schema identifier.
    fn metadata_id() -> Id {
        Self::id()
    }

    fn describe() -> (TribleSet, MemoryBlobStore<Blake3>) {
        (TribleSet::new(), MemoryBlobStore::new())
    }
}

impl<T> ConstMetadata for T where T: ValueSchema {}

impl<S> Metadata for PhantomData<S>
where
    S: ConstMetadata,
{
    fn metadata_id(&self) -> Id {
        <S as ConstMetadata>::metadata_id()
    }

    fn describe(&self) -> (TribleSet, MemoryBlobStore<Blake3>) {
        <S as ConstMetadata>::describe()
    }
}

impl<T> Metadata for T
where
    T: ValueSchema,
{
    fn metadata_id(&self) -> Id {
        T::id()
    }

    fn describe(&self) -> (TribleSet, MemoryBlobStore<Blake3>) {
        (TribleSet::new(), MemoryBlobStore::new())
    }
}
// namespace constants

pub const ATTR_VALUE_SCHEMA: Id = id_hex!("213F89E3F49628A105B3830BD3A6612C");
pub const ATTR_NAME: Id = id_hex!("2E26F8BA886495A8DF04ACF0ED3ACBD4");

attributes! {
    "2E26F8BA886495A8DF04ACF0ED3ACBD4" as name: valueschemas::ShortString;
    "213F89E3F49628A105B3830BD3A6612C" as attr_value_schema: valueschemas::GenId;
    /// Generic tag edge: link any entity to a tag entity (by Id). Reusable across domains.
    "91C50E9FBB1F73E892EBD5FFDE46C251" as tag: valueschemas::GenId;
}
