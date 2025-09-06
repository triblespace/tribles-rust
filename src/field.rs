//! Field helper type used by the query macros.
//!
//! The `Field<S>` type is a small, const-friendly wrapper around a 16-byte
//! attribute id (RawId) and a phantom type parameter `S` indicating the value
//! schema for that attribute. We keep construction simple and const-friendly so
//! fields can be declared as `pub const F: Field<ShortString> = Field::from(hex!("..."));`.

use core::marker::PhantomData;

use crate::id::RawId;
use crate::value::ValueSchema;

/// A typed reference to an attribute id together with its value schema.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Field<S: ValueSchema> {
    raw: RawId,
    _schema: PhantomData<S>,
}

impl<S: ValueSchema> Field<S> {
    /// Construct a `Field` from a raw 16-byte id.
    ///
    /// This is a `const fn` so it can be used in `const`/`static` declarations.
    pub const fn from(raw: RawId) -> Self {
        Self {
            raw,
            _schema: PhantomData,
        }
    }

    /// Return the underlying raw id bytes.
    pub const fn raw(&self) -> RawId {
        self.raw
    }

    /// Convert to a runtime `Id` value. This performs the nil check and will
    /// panic if the raw id is the nil id (all zeros).
    pub fn id(&self) -> crate::id::Id {
        crate::id::Id::new(self.raw).unwrap()
    }
}

pub use crate::id::RawId as RawIdAlias;

