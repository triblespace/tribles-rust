//! Field helper type used by the query macros.
//!
//! The `Field<S>` type is a small, const-friendly wrapper around a 16-byte
//! attribute id (RawId) and a phantom type parameter `S` indicating the value
//! schema for that attribute. We keep construction simple and const-friendly so
//! fields can be declared as `pub const F: Field<ShortString> = Field::from(hex!("..."));`.

use core::marker::PhantomData;

use crate::blob::ToBlob;
use crate::id::RawId;
use crate::value::schemas::hash::Blake3;
use crate::value::ValueSchema;
use blake3::Hasher;

/// A typed reference to an attribute id together with its value schema.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Attribute<S: ValueSchema> {
    raw: RawId,
    _schema: PhantomData<S>,
}

impl<S: ValueSchema> Attribute<S> {
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

    /// Convert a host value into a typed Value<S> using the Field's schema.
    /// This is a small convenience wrapper around the `ToValue` trait and
    /// simplifies macro expansion: `af.value_from(expr)` preserves the
    /// schema `S` for type inference.
    pub fn value_from<T: crate::value::ToValue<S>>(&self, v: T) -> crate::value::Value<S> {
        crate::value::ToValue::to_value(v)
    }

    /// Coerce an existing variable of any schema into a variable typed with
    /// this field's schema. This is a convenience for macros: they can
    /// allocate an untyped/UnknownValue variable and then annotate it with the
    /// field's schema using `af.as_variable(raw_var)`.
    ///
    /// The operation is a zero-cost conversion as variables are simply small
    /// integer indexes; the implementation uses an unsafe transmute to change
    /// the type parameter without moving the underlying data.
    pub fn as_variable(&self, v: crate::query::Variable<S>) -> crate::query::Variable<S> {
        v
    }

    /// Derive an attribute id from a dynamic field name and this schema's metadata.
    ///
    /// The identifier is computed by hashing the field name handle produced as a
    /// `Handle<Blake3, crate::blob::schemas::longstring::LongString>` together with the
    /// schema's [`ValueSchema::VALUE_SCHEMA_ID`] and [`ValueSchema::BLOB_SCHEMA_ID`].
    /// The resulting 32-byte Blake3 digest is truncated to 16 bytes to match the
    /// `RawId` layout used by [`Attribute::from`].
    pub fn from_field(field_name: &str) -> Self {
        let mut hasher = Hasher::new();

        let field_handle = String::from(field_name).to_blob().get_handle::<Blake3>();
        hasher.update(&field_handle.raw);
        hasher.update(S::VALUE_SCHEMA_ID.as_ref());

        let blob_schema_bytes: RawId = S::BLOB_SCHEMA_ID
            .map(|id| id.into())
            .unwrap_or([0; crate::id::ID_LEN]);
        hasher.update(&blob_schema_bytes);

        let digest = hasher.finalize();
        let mut raw = [0u8; crate::id::ID_LEN];
        raw.copy_from_slice(&digest.as_bytes()[..crate::id::ID_LEN]);
        Self::from(raw)
    }
}

pub use crate::id::RawId as RawIdAlias;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::schemas::longstring::LongString;
    use crate::value::schemas::hash::{Blake3, Handle};
    use crate::value::schemas::shortstring::ShortString;

    #[test]
    fn dynamic_field_is_deterministic() {
        let a1 = Attribute::<ShortString>::from_field("title");
        let a2 = Attribute::<ShortString>::from_field("title");

        assert_eq!(a1, a2);
        assert_ne!(a1.raw(), [0; crate::id::ID_LEN]);
    }

    #[test]
    fn dynamic_field_changes_with_name() {
        let title = Attribute::<ShortString>::from_field("title");
        let author = Attribute::<ShortString>::from_field("author");

        assert_ne!(title, author);
    }

    #[test]
    fn dynamic_field_includes_blob_schema() {
        let short = Attribute::<ShortString>::from_field("title");
        let handle = Attribute::<Handle<Blake3, LongString>>::from_field("title");

        assert_ne!(short.raw(), handle.raw());
    }
}
