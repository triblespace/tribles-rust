//! Representation of a single knowledge graph edge.
//!
//! For layout details and edge semantics see the [Trible Structure](../book/src/deep-dive/trible-structure.md) chapter of the Tribles Book.

mod tribleset;

use std::convert::TryInto;

use crate::{
    id::{ExclusiveId, Id},
    patch::{KeyOrdering, KeySegmentation},
    value::{Value, ValueSchema},
};

pub use tribleset::TribleSet;

/// The length of a trible in bytes.
pub const TRIBLE_LEN: usize = 64;

/// The start index of the entity in a trible.
pub const E_START: usize = 0;
/// The end index of the entity in a trible (inclusive).
pub const E_END: usize = 15;

/// The start index of the attribute in a trible.
pub const A_START: usize = 16;
/// The end index of the attribute in a trible (inclusive).
pub const A_END: usize = 31;

/// The start index of the value in a trible.
pub const V_START: usize = 32;
/// The end index of the value in a trible (inclusive).
pub const V_END: usize = 63;

/// Fundamentally a trible is always a collection of 64 bytes.
pub type RawTrible = [u8; TRIBLE_LEN];

/// The trible is the fundamental unit of storage in the knowledge graph,
/// and is stored in [crate::trible::TribleSet]s which index the trible in various ways,
/// allowing for efficient querying and retrieval of data.
///
/// ``` text
/// ┌────────────────────────────64 byte───────────────────────────┐
/// ┌──────────────┐┌──────────────┐┌──────────────────────────────┐
/// │  entity-id   ││ attribute-id ││        inlined value         │
/// └──────────────┘└──────────────┘└──────────────────────────────┘
/// └────16 byte───┘└────16 byte───┘└────────────32 byte───────────┘
/// ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─▶
/// ```
///
/// On a high level, a trible is a triple consisting of an entity, an attribute, and a value.
/// The entity and attribute are both 128-bit abstract extrinsic identifiers as described in [crate::id],
/// while the value is an arbitrary 256-bit [crate::value::Value].
/// The design of tribles is influenced by the need to minimize entropy while ensuring collision resistance.
/// Entities are abstract because they might have additional facts associated with them in the form of new tribles.
/// Similarly, attributes are abstract because their meaning is inherently non-grounded; the meaning of the "symbol" is only
/// the meaning ascribed to it, without any natural meaning.
/// Values can be any data that fits "inlined" into the fixed width, and they need to be large enough to hold an intrinsic
/// identifier for larger data. As established in the `id` module documentation, these need to be at least 256 bits / 32 bytes.
/// Counter-intuitively, their size and thus the size of "inline" data is determined by the scenario where data is too large
/// to be inlined. See [blob](crate::blob)s for a way to store larger data.
///
/// The trible is stored as a contiguous 64-byte array, with the entity taking the first 16 bytes,
/// the attribute taking the next 16 bytes, and the value taking the last 32 bytes.
///
/// The name trible is a portmanteau of triple and byte, and is pronounced like "tribble" from Star Trek.
/// This is also the reason why the mascot of the knowledge graph is Robert the tribble.
///
/// The minimalistic design of the trible has a number of advantages:
/// - It is very easy to define an order on tribles, which allows for efficient storage
///   and easy canonicalization of data.
/// - It is very easy to define a segmentation on tribles, which allows for efficient
///   indexing and querying of data, without then need for an interning mechanism,
///   that translates values to an internal integer representation. This simplifies
///   the implementation, saves memory and an additional lookup, prevents the single
///   registry from becoming a bottleneck, allowing for easy parallelization, and
///   obviates the need for a garbage collection mechanism.
/// - It is very easy to define a schema for the value, which allows for efficient
///   serialization and deserialization of data.
/// - On a high level, it is very easy to reason about the data stored in the knowledge graph.
///   Additionally, it is possible to estimate the physical size of the data stored in the knowledge graph
///   in terms of the number of bytes, as a function of the number of tribles stored.
/// - Due to the fundamental principles of minimizing entropy and ensuring collision resistance, it is likely that this format
///   will be independently discovered through convergent evolution, making it a strong candidate for a universal data interchange format.
///   And who knows, it might even be useful if we ever make contact with extra-terrestrial intelligences!
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Trible {
    pub data: RawTrible,
}

impl Trible {
    /// Creates a new trible from an entity, an attribute, and a value.
    ///
    /// # Arguments
    ///
    /// * `e` - The entity of the trible.
    /// * `a` - The attribute of the trible.
    /// * `v` - The value of the trible.
    ///
    /// # Returns
    ///
    /// A new trible.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    /// use valueschemas::R256;
    ///
    /// let e = fucid();
    /// let a = fucid();
    /// let v: Value<R256> = R256::value_from(42);
    /// let trible = Trible::new(&e, &a, &v);
    /// ```
    pub fn new<V: ValueSchema>(e: &ExclusiveId, a: &Id, v: &Value<V>) -> Trible {
        let mut data = [0; TRIBLE_LEN];
        data[E_START..=E_END].copy_from_slice(&e[..]);
        data[A_START..=A_END].copy_from_slice(&a[..]);
        data[V_START..=V_END].copy_from_slice(&v.raw[..]);

        Self { data }
    }

    /// Creates a new trible from an entity, an attribute, and a value.
    /// This is similar to [Trible::new], but takes a plain entity id instead of an owned id.
    /// Allowing to circumvent the ownership system, which can be used to inject
    /// data into a local knowledge graph without owning the entity.
    /// This is useful for loading existing trible data, for example when loading
    /// an existing [crate::trible::TribleSet] from a blob, or when declaring
    /// a namespace.
    ///
    /// # Arguments
    ///
    /// * `e` - The entity of the trible.
    /// * `a` - The attribute of the trible.
    /// * `v` - The value of the trible.
    ///
    /// # Returns
    ///
    /// A new trible.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    /// use valueschemas::R256;
    ///
    /// let e = fucid();
    /// let a = fucid();
    /// let v: Value<R256> = R256::value_from(42);
    /// let trible = Trible::force(&e, &a, &v);
    ///
    /// assert_eq!(trible.e(), &*e);
    /// ```
    pub fn force<V: ValueSchema>(e: &Id, a: &Id, v: &Value<V>) -> Trible {
        Trible::new(&ExclusiveId::as_transmute_force(e), a, v)
    }

    /// Creates a new trible from a raw trible (a 64-byte array).
    /// It circumvents the ownership system, and is useful for loading existing trible data,
    /// just like [Trible::force].
    ///
    /// # Arguments
    ///
    /// * `data` - The raw trible.
    ///
    /// # Returns
    ///
    /// A new trible if the entity and attribute are not nil
    /// (i.e. they are not all zeroes), otherwise `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    ///
    /// let data = [
    ///    // Entity
    ///    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    ///    // Attribute
    ///    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ///    // Value
    ///    32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    ///    48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    /// ];
    /// let trible = Trible::force_raw(data);
    /// assert!(trible.is_some());
    /// ```
    pub fn force_raw(data: RawTrible) -> Option<Trible> {
        if data[E_START..=E_END].iter().all(|&x| x == 0)
            || data[A_START..=A_END].iter().all(|&x| x == 0)
        {
            return None;
        }
        Some(Self { data })
    }

    /// Transmutes a raw trible reference into a trible reference.
    /// Circumvents the ownership system, and is useful for loading existing trible data,
    /// just like [Trible::force] and [Trible::force_raw].
    ///
    /// # Arguments
    ///
    /// * `data` - The raw trible reference.
    ///
    /// # Returns
    ///
    /// A new trible reference if the entity and attribute are not nil
    /// (i.e. they are not all zeroes), otherwise `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    ///
    /// let data = [
    ///   // Entity
    ///   0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    ///   // Attribute
    ///   16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ///   // Value
    ///   32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    ///   48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    /// ];
    /// let trible = Trible::as_transmute_force_raw(&data);
    /// assert!(trible.is_some());
    /// ```
    pub fn as_transmute_force_raw(data: &RawTrible) -> Option<&Self> {
        if data[E_START..=E_END].iter().all(|&x| x == 0)
            || data[A_START..=A_END].iter().all(|&x| x == 0)
        {
            return None;
        }
        Some(unsafe { std::mem::transmute(data) })
    }

    /// Transmutes a raw trible reference into a trible reference.
    /// Circumvents the ownership system, and does not check if the entity and attribute are nil.
    /// Should only be used if it it certain that the `RawTrible` is actually valid.
    pub fn as_transmute_raw_unchecked(data: &RawTrible) -> &Self {
        unsafe { std::mem::transmute(data) }
    }

    /// Returns the entity of the trible.
    ///
    /// # Returns
    ///
    /// The entity of the trible.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    ///
    /// let data = [
    ///   // Entity
    ///   0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    ///   // Attribute
    ///   16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ///   // Value
    ///   32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    ///   48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    /// ];
    /// let trible = Trible::force_raw(data).unwrap();
    /// let entity = trible.e();
    /// assert_eq!(entity, &Id::new([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]).unwrap());
    /// ```
    pub fn e(&self) -> &Id {
        Id::as_transmute_raw(self.data[E_START..=E_END].try_into().unwrap()).unwrap()
    }

    /// Returns the attribute of the trible.
    ///
    /// # Returns
    ///
    /// The attribute of the trible.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    ///
    /// let data = [
    ///   // Entity
    ///   0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    ///   // Attribute
    ///   16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ///   // Value
    ///   32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    ///   48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    /// ];
    /// let trible = Trible::force_raw(data).unwrap();
    /// let attribute = trible.a();
    /// assert_eq!(attribute, &Id::new([16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31]).unwrap());
    /// ```
    pub fn a(&self) -> &Id {
        Id::as_transmute_raw(self.data[A_START..=A_END].try_into().unwrap()).unwrap()
    }

    /// Returns the value of the trible.
    ///
    /// # Returns
    ///
    /// The value of the trible.
    ///
    /// # Example
    ///
    /// ```
    /// use tribles::prelude::*;
    /// use valueschemas::R256;
    ///
    /// let data = [
    ///   // Entity
    ///   0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    ///   // Attribute
    ///   16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ///   // Value
    ///   32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    ///   48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    /// ];
    /// let trible = Trible::force_raw(data).unwrap();
    /// let value = trible.v::<R256>();
    /// assert_eq!(value, &Value::new([32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    /// 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63]));
    /// ```
    pub fn v<V: ValueSchema>(&self) -> &Value<V> {
        Value::as_transmute_raw(self.data[V_START..=V_END].try_into().unwrap())
    }
}

/// A segmentation of the trible into three segments: entity, attribute, and value.
/// The entity is the first 16 bytes, the attribute is the next 16 bytes, and the value is the last 32 bytes.
/// This is used by the [crate::patch::PATCH] to efficiently index and query data in the [crate::trible::TribleSet].
///
/// This is a type-level constant and never instantiated.
#[derive(Copy, Clone, Debug)]
pub struct TribleSegmentation {}

impl KeySegmentation<TRIBLE_LEN> for TribleSegmentation {
    fn segment(depth: usize) -> usize {
        unsafe {
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                2, 2, 2, 2, 2, 2, 2, 2,
            ]
            .get_unchecked(depth)
            .clone()
        }
    }
}

/// An ordering of the trible with the segments in the order entity, attribute, value.
/// This is used by the [crate::patch::PATCH] to efficiently index and query data in the [crate::trible::TribleSet].
///
/// This is a type-level constant and never instantiated.
#[derive(Copy, Clone, Debug)]
pub struct EAVOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for EAVOrder {
    fn tree_index(key_index: usize) -> usize {
        key_index
    }

    fn key_index(tree_index: usize) -> usize {
        tree_index
    }
}

/// An ordering of the trible with the segments in the order entity, value, attribute.
/// This is used by the [crate::patch::PATCH] to efficiently index and query data in the [crate::trible::TribleSet].
///
/// This is a type-level constant and never instantiated.
#[derive(Copy, Clone, Debug)]
pub struct EVAOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for EVAOrder {
    fn tree_index(key_index: usize) -> usize {
        unsafe {
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 48, 49, 50, 51, 52, 53, 54,
                55, 56, 57, 58, 59, 60, 61, 62, 63, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
                28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            ]
            .get_unchecked(key_index)
            .clone()
        }
    }

    fn key_index(tree_index: usize) -> usize {
        unsafe {
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 32, 33, 34, 35, 36, 37, 38,
                39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
                60, 61, 62, 63, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            ]
            .get_unchecked(tree_index)
            .clone()
        }
    }
}

/// An ordering of the trible with the segments in the order attribute, entity, value.
/// This is used by the [crate::patch::PATCH] to efficiently index and query data in the [crate::trible::TribleSet].
///
/// This is a type-level constant and never instantiated.
#[derive(Copy, Clone, Debug)]
pub struct AEVOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for AEVOrder {
    fn tree_index(key_index: usize) -> usize {
        unsafe {
            [
                16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5,
                6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43,
                44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            ]
            .get_unchecked(key_index)
            .clone()
        }
    }

    fn key_index(tree_index: usize) -> usize {
        unsafe {
            [
                16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5,
                6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43,
                44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            ]
            .get_unchecked(tree_index)
            .clone()
        }
    }
}

/// An ordering of the trible with the segments in the order attribute, value, entity.
/// This is used by the [crate::patch::PATCH] to efficiently index and query data in the [crate::trible::TribleSet].
///
/// This is a type-level constant and never instantiated.
#[derive(Copy, Clone, Debug)]
pub struct AVEOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for AVEOrder {
    fn tree_index(key_index: usize) -> usize {
        unsafe {
            [
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 0, 1, 2, 3, 4, 5,
                6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
                28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            ]
            .get_unchecked(key_index)
            .clone()
        }
    }

    fn key_index(tree_index: usize) -> usize {
        unsafe {
            [
                16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
                37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57,
                58, 59, 60, 61, 62, 63, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            ]
            .get_unchecked(tree_index)
            .clone()
        }
    }
}

/// An ordering of the trible with the segments in the order value, entity, attribute.
/// This is used by the [crate::patch::PATCH] to efficiently index and query data in the [crate::trible::TribleSet].
///
/// This is a type-level constant and never instantiated.
#[derive(Copy, Clone, Debug)]
pub struct VEAOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for VEAOrder {
    fn tree_index(key_index: usize) -> usize {
        unsafe {
            [
                32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52,
                53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
                12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            ]
            .get_unchecked(key_index)
            .clone()
        }
    }

    fn key_index(tree_index: usize) -> usize {
        unsafe {
            [
                32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52,
                53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
                12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            ]
            .get_unchecked(tree_index)
            .clone()
        }
    }
}

/// An ordering of the trible with the segments in the order value, attribute, entity.
/// This is used by the [crate::patch::PATCH] to efficiently index and query data in the [crate::trible::TribleSet].
///
/// This is a type-level constant and never instantiated.
#[derive(Copy, Clone, Debug)]
pub struct VAEOrder {}

impl<const KEY_LEN: usize> KeyOrdering<KEY_LEN> for VAEOrder {
    fn tree_index(key_index: usize) -> usize {
        unsafe {
            [
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 32, 33, 34, 35, 36,
                37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
                12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            ]
            .get_unchecked(key_index)
            .clone()
        }
    }

    fn key_index(tree_index: usize) -> usize {
        unsafe {
            [
                32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52,
                53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
                26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            ]
            .get_unchecked(tree_index)
            .clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    #[test]
    fn order_eav() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        assert_eq!(EAVOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(EAVOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_eva() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ];
        assert_eq!(EVAOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(EVAOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_aev() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7,
            8, 9, 10, 11, 12, 13, 14, 15, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        assert_eq!(AEVOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(AEVOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_ave() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
            38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
            60, 61, 62, 63, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ];
        assert_eq!(AVEOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(AVEOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_vea() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ]; 
        let tree_bytes = [
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ];
        assert_eq!(VEAOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(VEAOrder::key_ordered(&tree_bytes), key_bytes);
    }

    #[rustfmt::skip]
    #[test]
    fn order_vae() {
        let key_bytes = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let tree_bytes = [
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ];
        assert_eq!(VAEOrder::tree_ordered(&key_bytes), tree_bytes);
        assert_eq!(VAEOrder::key_ordered(&tree_bytes), key_bytes);
    }
}
