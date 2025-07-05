//! Value type and conversion traits for schema types.
//!
//! # Portability & Common Formats
//! In order to build a portable and extensible database, we need to be able to store and retrieve arbitrary Rust types.
//! However, we can't just store the raw bytes of a type, because the bytes of a type are not portable across different platforms,
//! programming languages, or even different versions of the same type.
//!
//! We therefore need to define portable types that can be exchanged between different systems and expressed in a common format, i.e. bytes or strings.
//! RDF choose to use URIs for this purpose, which are easily readable and writable by humans, but not very efficient for machines.
//!
//! Our approach is to use a 32-byte array as the common format for all types, as it is the smallest amount of bits that provides sufficient entropy
//! to store intrinsic identifiers, i.e. hashes, in case we want to store larger values in a separate storage, i.e. a blob.
//!
//! We call these portable types "schemas", because they define the meaning and valid bit patterns of the bytes.
//! This allows us to use one common type, the [Value] type with a type parameter that represents the schema type, to store and retrieve arbitrary Rust types.
//!
//! The bytes are opaque to the Value type, and don't have to be valid for the schema type.
//! In cases where not all bit patterns are valid, conversion functions should check for validity via the [TryFromValue] trait.
//!
//! Conversion functions have to be implemented for the schema type and not for the [Value] type,
//! to circumvent the orphan rule when implementing conversion functions for types or schemas from other crates.
//!
//! To implement a new schema, create a new type that implements the [ValueSchema] trait.
//! Conversion functions can be implemented for the abstract schema type and concrete rust types
//! via the [ToValue], [FromValue], [TryToValue], and [TryFromValue] traits.
//!
//! Each schema type has to have a unique value schema id, and can optionally have a unique blob schema id.
//! The value schema id is used to identify the schema type for the bytes of a [Value] type, and the blob schema id is used to identify the schema type for the bytes of
//! the associated blob type, should the value schema represent a intrinsic identifier for a larger value stored in a blob. See the [crate::blob] module for more information.
//!
//! These ids are used to store metadata and documentation about the schema type in the knowledge graph itself,
//! and can for example be used to look up the schema type in a schema registry.
//!
//! Note that (de)serialization is essentialy a memory safe form of transmutation, and should be used with caution.
//! I.e. conversion between values of different schema types won't produce undefined behavior, but might produce unexpected results.
//!
//! # Example
//!
//! ```
//! use tribles::value::{Value, ValueSchema, ToValue, FromValue};
//! use tribles::id::Id;
//! use tribles::id_hex;
//! use std::convert::TryInto;
//!
//! // Define a new schema type.
//! // We're going to define an unsigned integer type that is stored as a little-endian 32-byte array.
//! // Note that makes our example easier, as we don't have to worry about sign-extension or padding bytes.
//! pub struct MyNumber;
//!
//! // Implement the ValueSchema trait for the schema type.
//! impl ValueSchema for MyNumber {
//!    const VALUE_SCHEMA_ID: Id = id_hex!("345EAC0C5B5D7D034C87777280B88AE2");
//! }
//!
//! // Implement conversion functions for the schema type.
//! impl FromValue<'_, MyNumber> for u32 {
//!    fn from_value(v: &Value<MyNumber>) -> Self {
//!      // Convert the schema type to the Rust type.
//!     u32::from_le_bytes(v.raw[0..4].try_into().unwrap())
//!  }
//! }
//!
//! impl ToValue<MyNumber> for u32 {
//!   fn to_value(self) -> Value<MyNumber> {
//!      // Convert the Rust type to the schema type, i.e. a 32-byte array.
//!      let mut bytes = [0; 32];
//!      bytes[0..4].copy_from_slice(&self.to_le_bytes());
//!      Value::new(bytes)
//!   }
//! }
//!
//! // Use the schema type to store and retrieve a Rust type.
//! let value: Value<MyNumber> = MyNumber::value_from(42u32);
//! let i: u32 = value.from_value();
//! assert_eq!(i, 42);
//!
//! // You can also implement conversion functions for other Rust types.
//! impl FromValue<'_, MyNumber> for u64 {
//!   fn from_value(v: &Value<MyNumber>) -> Self {
//!    u64::from_le_bytes(v.raw[0..8].try_into().unwrap())
//!   }
//! }
//!
//! impl ToValue<MyNumber> for u64 {
//!  fn to_value(self) -> Value<MyNumber> {
//!   let mut bytes = [0; 32];
//!   bytes[0..8].copy_from_slice(&self.to_le_bytes());
//!   Value::new(bytes)
//!   }
//! }
//!
//! let value: Value<MyNumber> = MyNumber::value_from(42u64);
//! let i: u64 = value.from_value();
//! assert_eq!(i, 42);
//!
//! // And use a value round-trip to convert between Rust types.
//! let value: Value<MyNumber> = MyNumber::value_from(42u32);
//! let i: u64 = value.from_value();
//! assert_eq!(i, 42);
//! ```

pub mod schemas;

use crate::id::Id;

use core::fmt;
use std::{borrow::Borrow, cmp::Ordering, fmt::Debug, hash::Hash, marker::PhantomData};

use hex::ToHex;
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes, Unaligned};

/// The length of a value in bytes.
pub const VALUE_LEN: usize = 32;

/// A raw value is simply a 32-byte array.
pub type RawValue = [u8; VALUE_LEN];

/// A value is a 32-byte array that can be (de)serialized as a Rust type.
/// The schema type parameter is an abstract type that represents the meaning
/// and valid bit patterns of the bytes.
///
/// # Example
///
/// ```
/// use tribles::prelude::*;
/// use valueschemas::R256;
/// use num_rational::Ratio;
///
/// let ratio = Ratio::new(1, 2);
/// let value: Value<R256> = R256::value_from(ratio);
/// let ratio2: Ratio<i128> = value.from_value();
/// assert_eq!(ratio, ratio2);
/// ```
#[derive(TryFromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct Value<T: ValueSchema> {
    pub raw: RawValue,
    _schema: PhantomData<T>,
}

impl<S: ValueSchema> Value<S> {
    /// Create a new value from a 32-byte array.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::value::{Value, ValueSchema};
    /// use tribles::value::schemas::UnknownValue;
    ///
    /// let bytes = [0; 32];
    /// let value = Value::<UnknownValue>::new(bytes);
    /// ```
    pub fn new(value: RawValue) -> Self {
        Self {
            raw: value,
            _schema: PhantomData,
        }
    }

    /// Transmute a value from one schema type to another.
    /// This is a safe operation, as the bytes are not changed.
    /// The schema type is only changed in the type system.
    /// This is a zero-cost operation.
    /// This is useful when you have a value with an abstract schema type,
    /// but you know the concrete schema type.
    pub fn transmute<O>(self) -> Value<O>
    where
        O: ValueSchema,
    {
        Value::new(self.raw)
    }

    /// Transmute a value reference from one schema type to another.
    /// This is a safe operation, as the bytes are not changed.
    /// The schema type is only changed in the type system.
    /// This is a zero-cost operation.
    /// This is useful when you have a value reference with an abstract schema type,
    /// but you know the concrete schema type.
    pub fn as_transmute<O>(&self) -> &Value<O>
    where
        O: ValueSchema,
    {
        unsafe { std::mem::transmute(self) }
    }

    /// Transmute a raw value reference to a value reference.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::value::{Value, ValueSchema};
    /// use tribles::value::schemas::UnknownValue;
    /// use std::borrow::Borrow;
    ///
    /// let bytes = [0; 32];
    /// let value: Value<UnknownValue> = Value::new(bytes);
    /// let value_ref: &Value<UnknownValue> = &value;
    /// let raw_value_ref: &[u8; 32] = value_ref.borrow();
    /// let value_ref2: &Value<UnknownValue> = Value::as_transmute_raw(raw_value_ref);
    /// assert_eq!(&value, value_ref2);
    /// ```
    pub fn as_transmute_raw(value: &RawValue) -> &Self {
        unsafe { std::mem::transmute(value) }
    }

    /// Deserialize a value with an abstract schema type to a concrete Rust type.
    ///
    /// Note that this may panic if the conversion is not possible.
    /// This might happen if the bytes are not valid for the schema type or if the
    /// rust type can't represent the specific value of the schema type,
    /// e.g. if the schema type is a fractional number and the rust type is an integer.
    ///
    /// For a conversion that always returns a result, use the [Value::try_from_value] method.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    /// use valueschemas::R256;
    /// use num_rational::Ratio;
    ///
    /// let value: Value<R256> = R256::value_from(Ratio::new(1, 2));
    /// let concrete: Ratio<i128> = value.from_value();
    /// ```
    pub fn from_value<'a, T>(&'a self) -> T
    where
        T: FromValue<'a, S>,
    {
        <T as FromValue<'a, S>>::from_value(self)
    }

    /// Deserialize a value with an abstract schema type to a concrete Rust type.
    ///
    /// This method returns an error if the conversion is not possible.
    /// This might happen if the bytes are not valid for the schema type or if the
    /// rust type can't represent the specific value of the schema type,
    /// e.g. if the schema type is a fractional number and the rust type is an integer.
    ///
    /// For a conversion that retrieves the value without error handling, use the [Value::from_value] method.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    /// use valueschemas::R256;
    /// use num_rational::Ratio;
    ///
    /// let value: Value<R256> = R256::value_from(Ratio::new(1, 2));
    /// let concrete: Result<Ratio<i128>, _> = value.try_from_value();
    /// ```
    ///
    pub fn try_from_value<'a, T>(&'a self) -> Result<T, <T as TryFromValue<'a, S>>::Error>
    where
        T: TryFromValue<'a, S>,
    {
        <T as TryFromValue<'a, S>>::try_from_value(self)
    }
}

impl<T: ValueSchema> Copy for Value<T> {}

impl<T: ValueSchema> Clone for Value<T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            _schema: PhantomData,
        }
    }
}

impl<T: ValueSchema> PartialEq for Value<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T: ValueSchema> Eq for Value<T> {}

impl<T: ValueSchema> Hash for Value<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T: ValueSchema> Ord for Value<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<T: ValueSchema> PartialOrd for Value<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: ValueSchema> Borrow<RawValue> for Value<S> {
    fn borrow(&self) -> &RawValue {
        &self.raw
    }
}

impl<T: ValueSchema> Debug for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Value<{}>({})",
            std::any::type_name::<T>(),
            ToHex::encode_hex::<String>(&self.raw)
        )
    }
}

/// A trait that represents an abstract schema type that can be (de)serialized as a [Value].
///
/// This trait is usually implemented on a type-level empty struct,
/// but may contain additional information about the schema type as associated constants or types.
/// The [Handle](crate::value::schemas::hash::Handle) type for example contains type information about the hash algorithm,
/// and the schema of the referenced blob.
///
/// See the [value](crate::value) module for more information.
/// See the [BlobSchema](crate::blob::BlobSchema) trait for the counterpart trait for blobs.
pub trait ValueSchema: Sized + 'static {
    const VALUE_SCHEMA_ID: Id;
    const BLOB_SCHEMA_ID: Option<Id> = None;

    /// Create a new value from a concrete Rust type.
    /// This is a convenience method that calls the [ToValue] trait.
    /// This method might panic if the conversion is not possible.
    ///
    /// See the [ValueSchema::value_try_from] method for a conversion that returns a result.
    fn value_from<T: ToValue<Self>>(t: T) -> Value<Self> {
        t.to_value()
    }

    /// Create a new value from a concrete Rust type.
    /// This is a convenience method that calls the [TryToValue] trait.
    /// This method might return an error if the conversion is not possible.
    ///
    /// See the [ValueSchema::value_from] method for a conversion that always succeeds (or panics).
    fn value_try_from<T: TryToValue<Self>>(
        t: T,
    ) -> Result<Value<Self>, <T as TryToValue<Self>>::Error> {
        t.try_to_value()
    }
}

/// A trait for converting a Rust type to a [Value] with a specific schema type.
/// This trait is implemented on the concrete Rust type.
///
/// This might cause a panic if the conversion is not possible,
/// see [TryToValue] for a conversion that returns a result.
///
/// This is the counterpart to the [FromValue] trait.
///
/// See [ToBlob](crate::blob::ToBlob) for the counterpart trait for blobs.
pub trait ToValue<S: ValueSchema> {
    /// Convert the Rust type to a [Value] with a specific schema type.
    /// This might cause a panic if the conversion is not possible.
    ///
    /// See the [TryToValue] trait for a conversion that returns a result.
    fn to_value(self) -> Value<S>;
}

/// A trait for converting a [Value] with a specific schema type to a Rust type.
/// This trait is implemented on the concrete Rust type.
///
/// This might cause a panic if the conversion is not possible,
/// see [TryFromValue] for a conversion that returns a result.
///
/// This is the counterpart to the [ToValue] trait.
///
/// See [TryFromBlob](crate::blob::TryFromBlob) for the counterpart trait for blobs.
pub trait FromValue<'a, S: ValueSchema> {
    /// Convert the [Value] with a specific schema type to the Rust type.
    /// This might cause a panic if the conversion is not possible.
    ///
    /// See the [TryFromValue] trait for a conversion that returns a result.
    fn from_value(v: &'a Value<S>) -> Self;
}

/// A trait for converting a Rust type to a [Value] with a specific schema type.
/// This trait is implemented on the concrete Rust type.
///
/// This might return an error if the conversion is not possible,
/// see [ToValue] for cases where the conversion is guaranteed to succeed (or panic).
///
/// This is the counterpart to the [TryFromValue] trait.
///
pub trait TryToValue<S: ValueSchema> {
    type Error;
    /// Convert the Rust type to a [Value] with a specific schema type.
    /// This might return an error if the conversion is not possible.
    ///
    /// See the [ToValue] trait for a conversion that always succeeds (or panics).
    fn try_to_value(self) -> Result<Value<S>, Self::Error>;
}

/// A trait for converting a [Value] with a specific schema type to a Rust type.
/// This trait is implemented on the concrete Rust type.
///
/// This might return an error if the conversion is not possible,
/// see [FromValue] for cases where the conversion is guaranteed to succeed (or panic).
///
/// This is the counterpart to the [TryToValue] trait.
///
/// See [TryFromBlob](crate::blob::TryFromBlob) for the counterpart trait for blobs.
pub trait TryFromValue<'a, S: ValueSchema>: Sized {
    type Error;
    /// Convert the [Value] with a specific schema type to the Rust type.
    /// This might return an error if the conversion is not possible.
    ///
    /// See the [FromValue] trait for a conversion that always succeeds (or panics).
    fn try_from_value(v: &'a Value<S>) -> Result<Self, Self::Error>;
}

impl<S: ValueSchema> ToValue<S> for Value<S> {
    fn to_value(self) -> Value<S> {
        self
    }
}

impl<S: ValueSchema> ToValue<S> for &Value<S> {
    fn to_value(self) -> Value<S> {
        *self
    }
}

impl<'a, S: ValueSchema> FromValue<'a, S> for Value<S> {
    fn from_value(v: &'a Value<S>) -> Self {
        *v
    }
}

impl<'a, S: ValueSchema> FromValue<'a, S> for () {
    fn from_value(_v: &'a Value<S>) -> Self {
        ()
    }
}
