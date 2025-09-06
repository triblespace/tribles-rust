//! Metadata namespace for the `tribles` crate.
//!
//! This namespace is used to bootstrap the meaning of other namespaces.
//! It defines meta attributes that are used to describe other attributes.

use crate::id::Id;
use crate::id_hex;
// namespace constants

pub const ATTR_VALUE_SCHEMA: Id = id_hex!("213F89E3F49628A105B3830BD3A6612C");
pub const ATTR_BLOB_SCHEMA: Id = id_hex!("02FAF947325161918C6D2E7D9DBA3485");
pub const ATTR_NAME: Id = id_hex!("2E26F8BA886495A8DF04ACF0ED3ACBD4");

pub mod metadata {
    #![allow(unused)]
    use crate::prelude::*;
    pub const name: crate::field::Field<crate::prelude::valueschemas::ShortString> =
        crate::field::Field::from(hex_literal::hex!("2E26F8BA886495A8DF04ACF0ED3ACBD4"));
    pub const attr_value_schema: crate::field::Field<crate::prelude::valueschemas::GenId> =
        crate::field::Field::from(hex_literal::hex!("213F89E3F49628A105B3830BD3A6612C"));
    pub const attr_blob_schema: crate::field::Field<crate::prelude::valueschemas::GenId> =
        crate::field::Field::from(hex_literal::hex!("02FAF947325161918C6D2E7D9DBA3485"));
    // Generic tag edge: link any entity to a tag entity (by Id). Reusable across domains.
    // Id generated via `trible genid`: 91C50E9FBB1F73E892EBD5FFDE46C251
    pub const tag: crate::field::Field<crate::prelude::valueschemas::GenId> =
        crate::field::Field::from(hex_literal::hex!("91C50E9FBB1F73E892EBD5FFDE46C251"));
}
