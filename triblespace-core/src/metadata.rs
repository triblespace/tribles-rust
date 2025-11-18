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
    fn describe(&self) -> (TribleSet, MemoryBlobStore<Blake3>);
}

/// Helper trait for schema types that want to expose metadata without requiring an instance.
pub trait SchemaMetadata: ValueSchema {
    fn describe_schema() -> (TribleSet, MemoryBlobStore<Blake3>) {
        (TribleSet::new(), MemoryBlobStore::new())
    }
}

impl<T> SchemaMetadata for T where T: ValueSchema {}

impl<S> Metadata for PhantomData<S>
where
    S: SchemaMetadata,
{
    fn describe(&self) -> (TribleSet, MemoryBlobStore<Blake3>) {
        S::describe_schema()
    }
}

impl<T> Metadata for T
where
    T: SchemaMetadata,
{
    fn describe(&self) -> (TribleSet, MemoryBlobStore<Blake3>) {
        T::describe_schema()
    }
}
// namespace constants

pub const ATTR_VALUE_SCHEMA: Id = id_hex!("213F89E3F49628A105B3830BD3A6612C");
pub const ATTR_BLOB_SCHEMA: Id = id_hex!("02FAF947325161918C6D2E7D9DBA3485");
pub const ATTR_NAME: Id = id_hex!("2E26F8BA886495A8DF04ACF0ED3ACBD4");

attributes! {
    "2E26F8BA886495A8DF04ACF0ED3ACBD4" as name: valueschemas::ShortString;
    "213F89E3F49628A105B3830BD3A6612C" as attr_value_schema: valueschemas::GenId;
    "02FAF947325161918C6D2E7D9DBA3485" as attr_blob_schema: valueschemas::GenId;
    /// Generic tag edge: link any entity to a tag entity (by Id). Reusable across domains.
    "91C50E9FBB1F73E892EBD5FFDE46C251" as tag: valueschemas::GenId;
}
