use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

use serde_json::{Map, Value as JsonValue};

use crate::attribute::Attribute;
use crate::id::ufoid;
use crate::id::ExclusiveId;
use crate::id::Id;
use crate::id::RawId;
use crate::id::ID_LEN;
use crate::metadata;
use crate::trible::Trible;
use crate::trible::TribleSet;
use crate::value::schemas::genid::GenId;
use crate::value::schemas::hash::HashProtocol;
use crate::value::schemas::UnknownValue;
use crate::value::RawValue;
use crate::value::Value;
use crate::value::ValueSchema;

fn emit_attribute_metadata<S: ValueSchema>(field: &str, raw: RawId, cache: &mut TribleSet) {
    let id = Id::new(raw).expect("Attribute::from_field produced nil id");
    let entity = ExclusiveId::force(id);

    let name_value = metadata::name.value_from(field);
    let name = Trible::new(&entity, &metadata::name.id(), &name_value);
    cache.insert(&name);

    let schema_value = GenId::value_from(&S::VALUE_SCHEMA_ID);
    let schema = Trible::new(&entity, &metadata::attr_value_schema.id(), &schema_value);
    cache.insert(&schema);

    if let Some(blob_schema) = S::BLOB_SCHEMA_ID {
        let blob_value = GenId::value_from(&blob_schema);
        let blob = Trible::new(&entity, &metadata::attr_blob_schema.id(), &blob_value);
        cache.insert(&blob);
    }
}

/// Error raised while converting JSON documents into tribles.
#[derive(Debug)]
pub enum JsonImportError {
    /// Top-level document was a JSON primitive instead of an object.
    PrimitiveRoot,
    /// Failed to parse JSON text before conversion.
    Parse(serde_json::Error),
    /// Failed to encode a string field into the configured schema.
    EncodeString { field: String, source: EncodeError },
    /// Failed to encode a numeric field into the configured schema.
    EncodeNumber { field: String, source: EncodeError },
    /// Failed to encode a boolean field into the configured schema.
    EncodeBool { field: String, source: EncodeError },
}

impl fmt::Display for JsonImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrimitiveRoot => write!(f, "cannot import JSON primitives as the document root"),
            Self::Parse(err) => write!(f, "failed to parse JSON: {err}"),
            Self::EncodeString { field, source } => {
                write!(f, "failed to encode string field {field:?}: {source}")
            }
            Self::EncodeNumber { field, source } => {
                write!(f, "failed to encode number field {field:?}: {source}")
            }
            Self::EncodeBool { field, source } => {
                write!(f, "failed to encode boolean field {field:?}: {source}")
            }
        }
    }
}

impl std::error::Error for JsonImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PrimitiveRoot => None,
            Self::Parse(err) => Some(err),
            Self::EncodeString { source, .. }
            | Self::EncodeNumber { source, .. }
            | Self::EncodeBool { source, .. } => Some(source.as_error()),
        }
    }
}

/// Error returned by user supplied encoders when converting JSON primitives.
#[derive(Debug)]
pub struct EncodeError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl EncodeError {
    /// Creates a simple error message for encoder failures.
    pub fn message(message: impl Into<String>) -> Self {
        #[derive(Debug)]
        struct Message(String);

        impl fmt::Display for Message {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::error::Error for Message {}

        Self(Box::new(Message(message.into())))
    }

    fn as_error(&self) -> &(dyn std::error::Error + 'static) {
        self.0.as_ref()
    }

    /// Wraps an existing error inside an [`EncodeError`].
    pub fn from_error(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Box::new(err))
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0.as_ref(), f)
    }
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

/// Imports JSON objects into [`TribleSet`]s using configurable schema mappings.
///
/// The importer creates a fresh [`ExclusiveId`] for each JSON object and derives
/// attribute identifiers by hashing the field name together with the chosen
/// [`ValueSchema`]. Arrays are treated as multi-valued fields: every element is
/// converted independently while retaining the same attribute id.
///
/// String, number, and boolean primitives are converted through user supplied
/// encoder closures. Those closures can perform additional validation, look up
/// existing blobs, or allocate new ones in a repository before returning the
/// [`Value`] to store. Because the encoders receive the raw JSON values they
/// can stage blobs in whatever [`BlobStore`](crate::repo::BlobStore) backend the
/// caller chooses before handing back the corresponding handles. Nested objects
/// recurse automatically and are linked to their parent entity via a `GenId`
/// attribute derived from the field name. Callers can also supply their own id
/// generator through [`JsonImporter::with_id_generator`] when they need
/// deterministic or structured identifiers.
///
/// Each time the importer derives a new attribute id it caches the raw
/// identifier. After conversion completes it emits metadata describing the
/// field name, value schema, and optional blob schema so downstream queries can
/// recover the attribute definition.
pub struct JsonImporter<
    'enc,
    StringSchema,
    NumberSchema,
    BoolSchema,
    StringEncoder,
    NumberEncoder,
    BoolEncoder,
    IdGenerator,
> where
    StringSchema: ValueSchema,
    NumberSchema: ValueSchema,
    BoolSchema: ValueSchema,
    StringEncoder: FnMut(&str) -> Result<Value<StringSchema>, EncodeError> + 'enc,
    NumberEncoder: FnMut(&serde_json::Number) -> Result<Value<NumberSchema>, EncodeError> + 'enc,
    BoolEncoder: FnMut(bool) -> Result<Value<BoolSchema>, EncodeError> + 'enc,
    IdGenerator: FnMut() -> ExclusiveId,
{
    string_encoder: StringEncoder,
    number_encoder: NumberEncoder,
    bool_encoder: BoolEncoder,
    id_generator: IdGenerator,
    data: TribleSet,
    string_attributes: HashMap<String, RawId>,
    number_attributes: HashMap<String, RawId>,
    bool_attributes: HashMap<String, RawId>,
    genid_attributes: HashMap<String, RawId>,
    _schemas: PhantomData<(StringSchema, NumberSchema, BoolSchema)>,
    _lifetime: PhantomData<&'enc ()>,
}

impl<
        'enc,
        StringSchema,
        NumberSchema,
        BoolSchema,
        StringEncoder,
        NumberEncoder,
        BoolEncoder,
        IdGenerator,
    >
    JsonImporter<
        'enc,
        StringSchema,
        NumberSchema,
        BoolSchema,
        StringEncoder,
        NumberEncoder,
        BoolEncoder,
        IdGenerator,
    >
where
    StringSchema: ValueSchema,
    NumberSchema: ValueSchema,
    BoolSchema: ValueSchema,
    StringEncoder: FnMut(&str) -> Result<Value<StringSchema>, EncodeError> + 'enc,
    NumberEncoder: FnMut(&serde_json::Number) -> Result<Value<NumberSchema>, EncodeError> + 'enc,
    BoolEncoder: FnMut(bool) -> Result<Value<BoolSchema>, EncodeError> + 'enc,
    IdGenerator: FnMut() -> ExclusiveId,
{
    /// Creates a new importer using the supplied primitive encoders and id generator.
    pub fn with_id_generator(
        string_encoder: StringEncoder,
        number_encoder: NumberEncoder,
        bool_encoder: BoolEncoder,
        id_generator: IdGenerator,
    ) -> Self {
        Self {
            string_encoder,
            number_encoder,
            bool_encoder,
            id_generator,
            data: TribleSet::new(),
            string_attributes: HashMap::new(),
            number_attributes: HashMap::new(),
            bool_attributes: HashMap::new(),
            genid_attributes: HashMap::new(),
            _schemas: PhantomData,
            _lifetime: PhantomData,
        }
    }

    fn next_id(&mut self) -> ExclusiveId {
        (self.id_generator)()
    }

    fn string_attribute(&mut self, field: &str) -> Attribute<StringSchema> {
        let raw = if let Some(raw) = self.string_attributes.get(field) {
            *raw
        } else {
            let attr = Attribute::<StringSchema>::from_field(field);
            let raw = attr.raw();
            self.string_attributes.insert(field.to_owned(), raw);
            raw
        };
        Attribute::from(raw)
    }

    fn number_attribute(&mut self, field: &str) -> Attribute<NumberSchema> {
        let raw = if let Some(raw) = self.number_attributes.get(field) {
            *raw
        } else {
            let attr = Attribute::<NumberSchema>::from_field(field);
            let raw = attr.raw();
            self.number_attributes.insert(field.to_owned(), raw);
            raw
        };
        Attribute::from(raw)
    }

    fn bool_attribute(&mut self, field: &str) -> Attribute<BoolSchema> {
        let raw = if let Some(raw) = self.bool_attributes.get(field) {
            *raw
        } else {
            let attr = Attribute::<BoolSchema>::from_field(field);
            let raw = attr.raw();
            self.bool_attributes.insert(field.to_owned(), raw);
            raw
        };
        Attribute::from(raw)
    }

    fn genid_attribute(&mut self, field: &str) -> Attribute<GenId> {
        let raw = if let Some(raw) = self.genid_attributes.get(field) {
            *raw
        } else {
            let attr = Attribute::<GenId>::from_field(field);
            let raw = attr.raw();
            self.genid_attributes.insert(field.to_owned(), raw);
            raw
        };
        Attribute::from(raw)
    }

    /// Parses JSON text and imports it into a [`TribleSet`].
    pub fn import_str(&mut self, input: &str) -> Result<(), JsonImportError> {
        let value = serde_json::from_str::<JsonValue>(input).map_err(JsonImportError::Parse)?;
        self.import_value(&value)
    }

    /// Imports a previously parsed JSON [`Value`].
    ///
    /// Root documents can either be objects, which yield a single entity, or
    /// arrays of objects, which return one entity per element. Primitive roots
    /// are rejected.
    pub fn import_value(&mut self, value: &JsonValue) -> Result<(), JsonImportError> {
        let mut staged = TribleSet::new();

        match value {
            JsonValue::Object(object) => {
                let root = self.next_id();
                self.stage_object(root, object, &mut staged)?;
            }
            JsonValue::Array(elements) => {
                for element in elements {
                    let JsonValue::Object(object) = element else {
                        return Err(JsonImportError::PrimitiveRoot);
                    };
                    let root = self.next_id();
                    self.stage_object(root, object, &mut staged)?;
                }
            }
            _ => return Err(JsonImportError::PrimitiveRoot),
        }

        self.data.union(staged);
        Ok(())
    }

    /// Returns the accumulated data tribles imported so far.
    pub fn data(&self) -> &TribleSet {
        &self.data
    }

    /// Returns metadata describing every derived attribute.
    pub fn metadata(&self) -> TribleSet {
        self.cached_metadata()
    }

    /// Clears the accumulated data tribles while retaining cached attributes.
    pub fn clear_data(&mut self) {
        self.data = TribleSet::new();
    }

    /// Clears the accumulated data tribles and resets all cached attributes.
    pub fn clear(&mut self) {
        self.clear_data();
        self.string_attributes.clear();
        self.number_attributes.clear();
        self.bool_attributes.clear();
        self.genid_attributes.clear();
    }

    fn cached_metadata(&self) -> TribleSet {
        let mut metadata = TribleSet::new();

        for (field, raw) in &self.string_attributes {
            emit_attribute_metadata::<StringSchema>(field, *raw, &mut metadata);
        }

        for (field, raw) in &self.number_attributes {
            emit_attribute_metadata::<NumberSchema>(field, *raw, &mut metadata);
        }

        for (field, raw) in &self.bool_attributes {
            emit_attribute_metadata::<BoolSchema>(field, *raw, &mut metadata);
        }

        for (field, raw) in &self.genid_attributes {
            emit_attribute_metadata::<GenId>(field, *raw, &mut metadata);
        }

        metadata
    }

    fn stage_object(
        &mut self,
        entity: ExclusiveId,
        object: &Map<String, JsonValue>,
        staged: &mut TribleSet,
    ) -> Result<(), JsonImportError> {
        for (field, value) in object {
            self.stage_field(&entity, field, value, staged)?;
        }

        Ok(())
    }

    fn stage_field(
        &mut self,
        entity: &ExclusiveId,
        field: &str,
        value: &JsonValue,
        staged: &mut TribleSet,
    ) -> Result<(), JsonImportError> {
        match value {
            JsonValue::Null => Ok(()),
            JsonValue::Bool(flag) => {
                let attr = self.bool_attribute(field);
                let attr_id = attr.id();
                let encoded =
                    (self.bool_encoder)(*flag).map_err(|err| JsonImportError::EncodeBool {
                        field: field.to_owned(),
                        source: err,
                    })?;
                staged.insert(&Trible::new(entity, &attr_id, &encoded));
                Ok(())
            }
            JsonValue::Number(number) => {
                let attr = self.number_attribute(field);
                let attr_id = attr.id();
                let encoded =
                    (self.number_encoder)(number).map_err(|err| JsonImportError::EncodeNumber {
                        field: field.to_owned(),
                        source: err,
                    })?;
                staged.insert(&Trible::new(entity, &attr_id, &encoded));
                Ok(())
            }
            JsonValue::String(text) => {
                let attr = self.string_attribute(field);
                let attr_id = attr.id();
                let encoded =
                    (self.string_encoder)(text).map_err(|err| JsonImportError::EncodeString {
                        field: field.to_owned(),
                        source: err,
                    })?;
                staged.insert(&Trible::new(entity, &attr_id, &encoded));
                Ok(())
            }
            JsonValue::Array(elements) => {
                for element in elements {
                    self.stage_field(entity, field, element, staged)?;
                }
                Ok(())
            }
            JsonValue::Object(object) => {
                let child_id = self.next_id();
                let value = GenId::value_from(&child_id);
                self.stage_object(child_id, object, staged)?;
                let attr = self.genid_attribute(field);
                let attr_id = attr.id();
                staged.insert(&Trible::new(entity, &attr_id, &value));
                Ok(())
            }
        }
    }
}

impl<'enc, StringSchema, NumberSchema, BoolSchema, StringEncoder, NumberEncoder, BoolEncoder>
    JsonImporter<
        'enc,
        StringSchema,
        NumberSchema,
        BoolSchema,
        StringEncoder,
        NumberEncoder,
        BoolEncoder,
        fn() -> ExclusiveId,
    >
where
    StringSchema: ValueSchema,
    NumberSchema: ValueSchema,
    BoolSchema: ValueSchema,
    StringEncoder: FnMut(&str) -> Result<Value<StringSchema>, EncodeError> + 'enc,
    NumberEncoder: FnMut(&serde_json::Number) -> Result<Value<NumberSchema>, EncodeError> + 'enc,
    BoolEncoder: FnMut(bool) -> Result<Value<BoolSchema>, EncodeError> + 'enc,
{
    /// Creates a new importer using the supplied primitive encoders.
    pub fn new(
        string_encoder: StringEncoder,
        number_encoder: NumberEncoder,
        bool_encoder: BoolEncoder,
    ) -> Self {
        Self {
            string_encoder,
            number_encoder,
            bool_encoder,
            id_generator: ufoid,
            data: TribleSet::new(),
            string_attributes: HashMap::new(),
            number_attributes: HashMap::new(),
            bool_attributes: HashMap::new(),
            genid_attributes: HashMap::new(),
            _schemas: PhantomData,
            _lifetime: PhantomData,
        }
    }
}

/// Deterministic variant of [`JsonImporter`] that derives entity identifiers
/// from the attribute/value pairs of each object.
///
/// Collected pairs are hashed using the configurable [`HashProtocol`] and the
/// first 16 bytes of the digest become the entity id. Arrays behave as
/// multi-valued fields and nested objects recurse while contributing their
/// deterministically generated ids to the parent hash via `GenId` values.
/// Attribute identifiers are cached like the non-deterministic importer and the
/// cached entries are converted into metadata describing the field name and
/// schema after each run.
pub struct DeterministicJsonImporter<
    'enc,
    StringSchema,
    NumberSchema,
    BoolSchema,
    StringEncoder,
    NumberEncoder,
    BoolEncoder,
    Hasher = crate::value::schemas::hash::Blake3,
> where
    StringSchema: ValueSchema,
    NumberSchema: ValueSchema,
    BoolSchema: ValueSchema,
    StringEncoder: FnMut(&str) -> Result<Value<StringSchema>, EncodeError> + 'enc,
    NumberEncoder: FnMut(&serde_json::Number) -> Result<Value<NumberSchema>, EncodeError> + 'enc,
    BoolEncoder: FnMut(bool) -> Result<Value<BoolSchema>, EncodeError> + 'enc,
    Hasher: HashProtocol,
{
    string_encoder: StringEncoder,
    number_encoder: NumberEncoder,
    bool_encoder: BoolEncoder,
    data: TribleSet,
    string_attributes: HashMap<String, RawId>,
    number_attributes: HashMap<String, RawId>,
    bool_attributes: HashMap<String, RawId>,
    genid_attributes: HashMap<String, RawId>,
    id_salt: Option<[u8; 32]>,
    _schemas: PhantomData<(StringSchema, NumberSchema, BoolSchema)>,
    _hasher: PhantomData<Hasher>,
    _lifetime: PhantomData<&'enc ()>,
}

impl<
        'enc,
        StringSchema,
        NumberSchema,
        BoolSchema,
        StringEncoder,
        NumberEncoder,
        BoolEncoder,
        Hasher,
    >
    DeterministicJsonImporter<
        'enc,
        StringSchema,
        NumberSchema,
        BoolSchema,
        StringEncoder,
        NumberEncoder,
        BoolEncoder,
        Hasher,
    >
where
    StringSchema: ValueSchema,
    NumberSchema: ValueSchema,
    BoolSchema: ValueSchema,
    StringEncoder: FnMut(&str) -> Result<Value<StringSchema>, EncodeError> + 'enc,
    NumberEncoder: FnMut(&serde_json::Number) -> Result<Value<NumberSchema>, EncodeError> + 'enc,
    BoolEncoder: FnMut(bool) -> Result<Value<BoolSchema>, EncodeError> + 'enc,
    Hasher: HashProtocol,
{
    /// Creates a new deterministic importer using the supplied primitive encoders.
    pub fn new(
        string_encoder: StringEncoder,
        number_encoder: NumberEncoder,
        bool_encoder: BoolEncoder,
    ) -> Self {
        Self::new_with_salt(string_encoder, number_encoder, bool_encoder, None)
    }

    /// Creates a new deterministic importer using the supplied primitive
    /// encoders and an explicit optional 32-byte salt mixed into every derived
    /// entity ID.
    pub fn new_with_salt(
        string_encoder: StringEncoder,
        number_encoder: NumberEncoder,
        bool_encoder: BoolEncoder,
        salt: Option<[u8; 32]>,
    ) -> Self {
        Self {
            string_encoder,
            number_encoder,
            bool_encoder,
            data: TribleSet::new(),
            string_attributes: HashMap::new(),
            number_attributes: HashMap::new(),
            bool_attributes: HashMap::new(),
            genid_attributes: HashMap::new(),
            id_salt: salt,
            _schemas: PhantomData,
            _hasher: PhantomData,
            _lifetime: PhantomData,
        }
    }

    /// Parses JSON text and imports it deterministically into a [`TribleSet`].
    pub fn import_str(&mut self, input: &str) -> Result<(), JsonImportError> {
        let value = serde_json::from_str::<JsonValue>(input).map_err(JsonImportError::Parse)?;
        self.import_value(&value)
    }

    /// Imports a previously parsed JSON [`Value`].
    ///
    /// Root documents can either be objects, which yield a single entity, or
    /// arrays of objects, which return one entity per element. Primitive roots
    /// are rejected.
    pub fn import_value(&mut self, value: &JsonValue) -> Result<(), JsonImportError> {
        let mut staged = TribleSet::new();

        match value {
            JsonValue::Object(object) => {
                self.stage_object(object, &mut staged)?;
            }
            JsonValue::Array(elements) => {
                for element in elements {
                    let JsonValue::Object(object) = element else {
                        return Err(JsonImportError::PrimitiveRoot);
                    };
                    self.stage_object(object, &mut staged)?;
                }
            }
            _ => return Err(JsonImportError::PrimitiveRoot),
        }

        self.data.union(staged);
        Ok(())
    }

    /// Returns the accumulated data tribles imported so far.
    pub fn data(&self) -> &TribleSet {
        &self.data
    }

    /// Returns metadata describing every derived attribute.
    pub fn metadata(&self) -> TribleSet {
        self.cached_metadata()
    }

    /// Clears the accumulated data tribles while retaining cached attributes.
    pub fn clear_data(&mut self) {
        self.data = TribleSet::new();
    }

    /// Clears the accumulated data tribles and resets all cached attributes.
    pub fn clear(&mut self) {
        self.clear_data();
        self.string_attributes.clear();
        self.number_attributes.clear();
        self.bool_attributes.clear();
        self.genid_attributes.clear();
    }

    fn string_attribute(&mut self, field: &str) -> Attribute<StringSchema> {
        let raw = if let Some(raw) = self.string_attributes.get(field) {
            *raw
        } else {
            let attr = Attribute::<StringSchema>::from_field(field);
            let raw = attr.raw();
            self.string_attributes.insert(field.to_owned(), raw);
            raw
        };
        Attribute::from(raw)
    }

    fn number_attribute(&mut self, field: &str) -> Attribute<NumberSchema> {
        let raw = if let Some(raw) = self.number_attributes.get(field) {
            *raw
        } else {
            let attr = Attribute::<NumberSchema>::from_field(field);
            let raw = attr.raw();
            self.number_attributes.insert(field.to_owned(), raw);
            raw
        };
        Attribute::from(raw)
    }

    fn bool_attribute(&mut self, field: &str) -> Attribute<BoolSchema> {
        let raw = if let Some(raw) = self.bool_attributes.get(field) {
            *raw
        } else {
            let attr = Attribute::<BoolSchema>::from_field(field);
            let raw = attr.raw();
            self.bool_attributes.insert(field.to_owned(), raw);
            raw
        };
        Attribute::from(raw)
    }

    fn genid_attribute(&mut self, field: &str) -> Attribute<GenId> {
        let raw = if let Some(raw) = self.genid_attributes.get(field) {
            *raw
        } else {
            let attr = Attribute::<GenId>::from_field(field);
            let raw = attr.raw();
            self.genid_attributes.insert(field.to_owned(), raw);
            raw
        };
        Attribute::from(raw)
    }

    fn cached_metadata(&self) -> TribleSet {
        let mut metadata = TribleSet::new();

        for (field, raw) in &self.string_attributes {
            emit_attribute_metadata::<StringSchema>(field, *raw, &mut metadata);
        }

        for (field, raw) in &self.number_attributes {
            emit_attribute_metadata::<NumberSchema>(field, *raw, &mut metadata);
        }

        for (field, raw) in &self.bool_attributes {
            emit_attribute_metadata::<BoolSchema>(field, *raw, &mut metadata);
        }

        for (field, raw) in &self.genid_attributes {
            emit_attribute_metadata::<GenId>(field, *raw, &mut metadata);
        }

        metadata
    }

    fn stage_object(
        &mut self,
        object: &Map<String, JsonValue>,
        staged: &mut TribleSet,
    ) -> Result<ExclusiveId, JsonImportError> {
        let mut pairs = Vec::new();

        for (field, value) in object {
            self.stage_field(field, value, &mut pairs, staged)?;
        }

        let entity = self.derive_id(&pairs);

        for (attribute, value) in pairs {
            let attribute_id =
                Id::new(attribute).expect("deterministic importer produced nil attribute id");
            let encoded = Value::<UnknownValue>::new(value);
            staged.insert(&Trible::new(&entity, &attribute_id, &encoded));
        }

        Ok(entity)
    }

    fn stage_field(
        &mut self,
        field: &str,
        value: &JsonValue,
        pairs: &mut Vec<(RawId, RawValue)>,
        staged: &mut TribleSet,
    ) -> Result<(), JsonImportError> {
        match value {
            JsonValue::Null => Ok(()),
            JsonValue::Bool(flag) => {
                let attr = self.bool_attribute(field);
                let encoded =
                    (self.bool_encoder)(*flag).map_err(|err| JsonImportError::EncodeBool {
                        field: field.to_owned(),
                        source: err,
                    })?;
                pairs.push((attr.raw(), encoded.raw));
                Ok(())
            }
            JsonValue::Number(number) => {
                let attr = self.number_attribute(field);
                let encoded =
                    (self.number_encoder)(number).map_err(|err| JsonImportError::EncodeNumber {
                        field: field.to_owned(),
                        source: err,
                    })?;
                pairs.push((attr.raw(), encoded.raw));
                Ok(())
            }
            JsonValue::String(text) => {
                let attr = self.string_attribute(field);
                let encoded =
                    (self.string_encoder)(text).map_err(|err| JsonImportError::EncodeString {
                        field: field.to_owned(),
                        source: err,
                    })?;
                pairs.push((attr.raw(), encoded.raw));
                Ok(())
            }
            JsonValue::Array(elements) => {
                for element in elements {
                    self.stage_field(field, element, pairs, staged)?;
                }
                Ok(())
            }
            JsonValue::Object(object) => {
                let child_entity = self.stage_object(object, staged)?;
                let attr = self.genid_attribute(field);
                let value = GenId::value_from(&child_entity);
                pairs.push((attr.raw(), value.raw));
                Ok(())
            }
        }
    }

    fn derive_id(&self, values: &[(RawId, RawValue)]) -> ExclusiveId {
        let mut pairs = values.to_vec();
        pairs.sort_by(|(attr_a, value_a), (attr_b, value_b)| {
            attr_a.cmp(attr_b).then(value_a.cmp(value_b))
        });

        let mut hasher = Hasher::new();
        if let Some(salt) = self.id_salt {
            hasher.update(salt.as_ref());
        }
        for (attribute, value) in &pairs {
            hasher.update(attribute);
            hasher.update(value);
        }

        let digest: [u8; 32] = hasher.finalize().into();
        let mut raw = [0u8; ID_LEN];
        raw.copy_from_slice(&digest[..ID_LEN]);
        let id = Id::new(raw).expect("deterministic importer produced nil id");

        ExclusiveId::force(id)
    }
}

#[cfg(test)]
mod tests {
    use core::num;

    use super::*;

    use crate::blob::schemas::longstring::LongString;
    use crate::blob::MemoryBlobStore;
    use crate::blob::ToBlob;
    use crate::id::fucid;
    use crate::id::Id;
    use crate::metadata;
    use crate::repo::BlobStore;
    use crate::value::ToValue;
    use crate::value::schemas::boolean::Boolean;
    use crate::value::schemas::f256::F256;
    use crate::value::schemas::hash::{Blake3, Handle};
    use crate::value::schemas::shortstring::ShortString;
    use crate::value::ValueSchema;
    use anybytes::View;
    use f256::f256;

    fn make_importer() -> JsonImporter<
        'static,
        Handle<Blake3, LongString>,
        F256,
        Boolean,
        impl FnMut(&str) -> Result<Value<Handle<Blake3, LongString>>, EncodeError>,
        impl FnMut(&serde_json::Number) -> Result<Value<F256>, EncodeError>,
        impl FnMut(bool) -> Result<Value<Boolean>, EncodeError>,
        fn() -> ExclusiveId,
    > {
        JsonImporter::new(
            |text: &str| Ok(ToBlob::<LongString>::to_blob(text.to_string()).get_handle::<Blake3>()),
            |number: &serde_json::Number| {
                if let Some(n) = number.as_u128() {
                    return Ok(f256::from(n).to_value());
                }
                if let Some(n) = number.as_i128() {
                    return Ok(f256::from(n).to_value());
                }
                if let Some(n) = number.as_f64() {
                    return Ok(f256::from(n).to_value());
                }
                Err(EncodeError::message("failed to decode JSON number"))
            },
            |flag: bool| Ok(Boolean::value_from(flag)),
        )
    }

    fn make_deterministic_importer() -> DeterministicJsonImporter<
        'static,
        Handle<Blake3, LongString>,
        F256,
        Boolean,
        impl FnMut(&str) -> Result<Value<Handle<Blake3, LongString>>, EncodeError>,
        impl FnMut(&serde_json::Number) -> Result<Value<F256>, EncodeError>,
        impl FnMut(bool) -> Result<Value<Boolean>, EncodeError>,
    > {
        make_deterministic_importer_with_salt(None)
    }

    fn make_deterministic_importer_with_salt(
        salt: Option<[u8; 32]>,
    ) -> DeterministicJsonImporter<
        'static,
        Handle<Blake3, LongString>,
        F256,
        Boolean,
        impl FnMut(&str) -> Result<Value<Handle<Blake3, LongString>>, EncodeError>,
        impl FnMut(&serde_json::Number) -> Result<Value<F256>, EncodeError>,
        impl FnMut(bool) -> Result<Value<Boolean>, EncodeError>,
    > {
        DeterministicJsonImporter::new_with_salt(
            |text: &str| Ok(ToBlob::<LongString>::to_blob(text.to_string()).get_handle::<Blake3>()),
            |number: &serde_json::Number| {
                if let Some(n) = number.as_u128() {
                    return Ok(f256::from(n).to_value());
                }
                if let Some(n) = number.as_i128() {
                    return Ok(f256::from(n).to_value());
                }
                if let Some(n) = number.as_f64() {
                    return Ok(f256::from(n).to_value());
                }
                Err(EncodeError::message("failed to decode JSON number"))
            },
            |flag: bool| Ok(Boolean::value_from(flag)),
            salt,
        )
    }

    #[test]
    fn salted_importer_changes_entity_ids() {
        let payload = serde_json::json!({ "title": "Dune" });

        let mut unsalted = make_deterministic_importer();
        unsalted.import_value(&payload).unwrap();
        let unsalted_root = *unsalted.data().iter().next().unwrap().e();

        let salt = [0x55; 32];
        let mut salted = make_deterministic_importer_with_salt(Some(salt));
        salted.import_value(&payload).unwrap();
        let salted_root = *salted.data().iter().next().unwrap().e();

        assert_ne!(unsalted_root, salted_root);

        let mut salted_again = make_deterministic_importer_with_salt(Some(salt));
        salted_again.import_value(&payload).unwrap();
        let salted_again_root = *salted_again.data().iter().next().unwrap().e();

        assert_eq!(salted_root, salted_again_root);
    }

    fn assert_attribute_metadata<S: ValueSchema>(metadata: &TribleSet, attribute: Id, field: &str) {
        let name_attr = metadata::name.id();
        let schema_attr = metadata::attr_value_schema.id();
        let blob_attr = metadata::attr_blob_schema.id();

        let entries: Vec<Trible> = metadata
            .iter()
            .filter(|trible| {
                *trible.e() == attribute
                    && (*trible.a() == name_attr
                        || *trible.a() == schema_attr
                        || *trible.a() == blob_attr)
            })
            .copied()
            .collect();

        assert!(
            entries.iter().any(|t| *t.a() == name_attr),
            "missing metadata::name for {field}"
        );
        assert!(
            entries.iter().any(|t| *t.a() == schema_attr),
            "missing metadata::attr_value_schema for {field}"
        );

        let name_value = entries
            .iter()
            .find(|t| *t.a() == name_attr)
            .expect("name metadata should exist")
            .v::<ShortString>()
            .from_value::<String>();
        assert_eq!(name_value, field);

        let schema_value = entries
            .iter()
            .find(|t| *t.a() == schema_attr)
            .expect("value schema metadata should exist")
            .v::<GenId>()
            .from_value::<Id>();
        assert_eq!(schema_value, S::VALUE_SCHEMA_ID);

        match S::BLOB_SCHEMA_ID {
            Some(expected) => {
                let blob = entries
                    .iter()
                    .find(|t| *t.a() == blob_attr)
                    .expect("blob schema metadata should exist")
                    .v::<GenId>()
                    .from_value::<Id>();
                assert_eq!(blob, expected);
            }
            None => {
                assert!(
                    entries.iter().all(|t| *t.a() != blob_attr),
                    "unexpected blob schema metadata for {field}"
                );
            }
        }
    }

    #[test]
    fn imports_flat_object() {
        let mut importer = make_importer();
        let payload = serde_json::json!({
            "title": "Dune",
            "pages": 412,
            "available": true,
            "tags": ["scifi", "classic"],
            "skip": null
        });

        importer.import_value(&payload).unwrap();
        let data: Vec<_> = importer.data().iter().collect();
        let metadata = importer.metadata();

        assert_eq!(data.len(), 5);
        let root = *data.first().unwrap().e();
        for trible in &data {
            assert_eq!(*trible.e(), root);
        }

        let title_attr = Attribute::<Handle<Blake3, LongString>>::from_field("title").id();
        let tags_attr = Attribute::<Handle<Blake3, LongString>>::from_field("tags").id();
        let pages_attr = Attribute::<F256>::from_field("pages").id();
        let available_attr = Attribute::<Boolean>::from_field("available").id();

        let mut tag_values = Vec::new();
        for trible in &data {
            let attribute = trible.a();
            if *attribute == title_attr {
                let value = trible.v::<Handle<Blake3, LongString>>();
                let expected = ToBlob::<LongString>::to_blob("Dune").get_handle::<Blake3>();
                assert_eq!(value.raw, expected.raw);
            } else if *attribute == tags_attr {
                tag_values.push(trible.v::<Handle<Blake3, LongString>>().raw);
            } else if *attribute == pages_attr {
                let value = trible.v::<F256>();
                let number: f256 = value.from_value();
                let expected = f256::from(412.0);
                assert_eq!(number, expected);
            } else if *attribute == available_attr {
                let value = trible.v::<Boolean>();
                assert!(value.from_value::<bool>());
            }
        }
        assert_eq!(tag_values.len(), 2);

        assert_attribute_metadata::<Handle<Blake3, LongString>>(&metadata, title_attr, "title");
        assert_attribute_metadata::<Handle<Blake3, LongString>>(&metadata, tags_attr, "tags");
        assert_attribute_metadata::<F256>(&metadata, pages_attr, "pages");
        assert_attribute_metadata::<Boolean>(&metadata, available_attr, "available");
    }

    #[test]
    fn imports_nested_objects() {
        let mut importer = make_importer();
        let payload = serde_json::json!({
            "title": "Dune",
            "author": {
                "first": "Frank",
                "last": "Herbert"
            }
        });

        importer.import_value(&payload).unwrap();
        let data: Vec<_> = importer.data().iter().collect();
        let metadata = importer.metadata();
        assert_eq!(data.len(), 4);

        let author_attr = Attribute::<GenId>::from_field("author").id();
        let mut child_ids = Vec::new();
        for trible in &data {
            if *trible.a() == author_attr {
                let child = trible.v::<GenId>().from_value::<ExclusiveId>();
                child_ids.push(child);
            }
        }
        assert_eq!(child_ids.len(), 1);
        let child_id = child_ids.into_iter().next().unwrap();

        let first_attr = Attribute::<Handle<Blake3, LongString>>::from_field("first").id();
        let last_attr = Attribute::<Handle<Blake3, LongString>>::from_field("last").id();

        let mut seen_first = false;
        let mut seen_last = false;
        for trible in &data {
            if *trible.e() == child_id.id {
                if *trible.a() == first_attr {
                    let value = trible.v::<Handle<Blake3, LongString>>();
                    let expected = ToBlob::<LongString>::to_blob("Frank").get_handle::<Blake3>();
                    assert_eq!(value.raw, expected.raw);
                    seen_first = true;
                } else if *trible.a() == last_attr {
                    let value = trible.v::<Handle<Blake3, LongString>>();
                    let expected = ToBlob::<LongString>::to_blob("Herbert").get_handle::<Blake3>();
                    assert_eq!(value.raw, expected.raw);
                    seen_last = true;
                }
            }
        }

        assert!(seen_first && seen_last);

        assert_attribute_metadata::<GenId>(&metadata, author_attr, "author");
        assert_attribute_metadata::<Handle<Blake3, LongString>>(&metadata, first_attr, "first");
        assert_attribute_metadata::<Handle<Blake3, LongString>>(&metadata, last_attr, "last");
    }

    #[test]
    fn imports_top_level_array() {
        let mut importer = make_importer();
        let payload = serde_json::json!([
            { "title": "Dune" },
            { "title": "Dune Messiah" }
        ]);

        importer.import_value(&payload).unwrap();
        let data: Vec<_> = importer.data().iter().collect();

        assert_eq!(data.len(), 2);

        let title_attr = Attribute::<Handle<Blake3, LongString>>::from_field("title").id();
        let mut by_root = std::collections::HashMap::new();
        for trible in &data {
            assert_eq!(trible.a(), &title_attr);
            by_root.insert(*trible.e(), trible.v::<Handle<Blake3, LongString>>().raw);
        }

        assert_eq!(by_root.len(), 2);

        let observed: std::collections::BTreeSet<_> = by_root.values().copied().collect();
        let expected: std::collections::BTreeSet<_> = ["Dune", "Dune Messiah"]
            .into_iter()
            .map(|title| {
                ToBlob::<LongString>::to_blob(title)
                    .get_handle::<Blake3>()
                    .raw
            })
            .collect();

        assert_eq!(observed, expected);
    }

    #[test]
    fn reports_encoder_errors_with_field() {
        let mut importer = JsonImporter::new(
            |text: &str| {
                if text.is_empty() {
                    return Err(EncodeError::message("empty strings are not allowed"));
                }
                Ok(ToBlob::<LongString>::to_blob(text.to_string()).get_handle::<Blake3>())
            },
            |number: &serde_json::Number| {
                let value = number.as_f64().ok_or_else(|| EncodeError::message("bad"))?;
                let converted: Value<F256> = f256::from(value).to_value();
                Ok(converted)
            },
            |flag: bool| Ok(Boolean::value_from(flag)),
        );

        let payload = serde_json::json!({ "name": "", "ok": true });
        let err = importer.import_value(&payload).unwrap_err();
        match err {
            JsonImportError::EncodeString { field, source } => {
                assert_eq!(field, "name");
                assert!(source.to_string().contains("empty"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn rejects_primitive_roots() {
        let mut importer = make_importer();
        let payload = serde_json::json!("nope");
        let err = importer.import_value(&payload).unwrap_err();
        match err {
            JsonImportError::PrimitiveRoot => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn string_encoder_can_write_to_blobstore() {
        let mut store: MemoryBlobStore<Blake3> = MemoryBlobStore::new();

        let mut importer = JsonImporter::new(
            |text: &str| {
                let blob = ToBlob::<LongString>::to_blob(text.to_string());
                Ok(store.insert(blob))
            },
            |_number: &serde_json::Number| -> Result<Value<F256>, EncodeError> {
                unreachable!("number encoder should not be called in this test");
            },
            |_flag: bool| -> Result<Value<Boolean>, EncodeError> {
                unreachable!("bool encoder should not be called in this test");
            },
        );

        let payload = serde_json::json!({ "description": "the spice must flow" });
        importer.import_value(&payload).unwrap();

        let data: Vec<_> = importer.data().iter().collect();
        let metadata = importer.metadata();
        assert_eq!(data.len(), 1);

        let description_attr =
            Attribute::<Handle<Blake3, LongString>>::from_field("description").id();
        let trible = data.first().unwrap();
        assert_eq!(trible.a(), &description_attr);
        let stored_value = trible.v::<Handle<Blake3, LongString>>().clone();

        let entries: Vec<_> = store.reader().unwrap().into_iter().collect();
        assert_eq!(entries.len(), 1);

        let (handle, blob) = &entries[0];
        let handle: Value<Handle<Blake3, LongString>> = handle.clone().transmute();
        assert_eq!(handle.raw, stored_value.raw);

        let text: View<str> = blob
            .clone()
            .transmute::<LongString>()
            .try_from_blob()
            .unwrap();
        assert_eq!(text.as_ref(), "the spice must flow");

        assert_attribute_metadata::<Handle<Blake3, LongString>>(
            &metadata,
            description_attr,
            "description",
        );
    }

    #[test]
    fn honors_custom_id_generator() {
        let mut ids = vec![fucid(), fucid()];
        let expected: Vec<_> = ids.iter().map(|id| id.id).collect();
        ids.reverse();

        let mut importer = JsonImporter::with_id_generator(
            |text: &str| Ok(ToBlob::<LongString>::to_blob(text.to_string()).get_handle::<Blake3>()),
            |number: &serde_json::Number| {
                let primitive = if let Some(n) = number.as_i64() {
                    n as f64
                } else if let Some(n) = number.as_u64() {
                    n as f64
                } else {
                    number
                        .as_f64()
                        .ok_or_else(|| EncodeError::message("non-finite JSON number"))?
                };
                let converted: Value<F256> = f256::from(primitive).to_value();
                Ok(converted)
            },
            |flag: bool| Ok(Boolean::value_from(flag)),
            move || ids.pop().expect("custom id generator exhausted"),
        );

        let payload = serde_json::json!({
            "title": "Dune",
            "author": {
                "first": "Frank"
            }
        });

        importer.import_value(&payload).unwrap();
        let data: Vec<_> = importer.data().iter().collect();

        let author_attr = Attribute::<GenId>::from_field("author").id();
        let mut root = None;
        let mut child = None;
        for trible in &data {
            if *trible.a() == author_attr {
                root = Some(*trible.e());
                child = Some(trible.v::<GenId>().from_value::<ExclusiveId>());
            }
        }

        let root = root.expect("missing root reference");
        assert_eq!(root, expected[0]);

        let child = child.expect("missing child reference");
        assert_eq!(child.id, expected[1]);
    }

    #[test]
    fn clear_resets_cached_attributes() {
        let mut importer = make_importer();
        let payload = serde_json::json!({
            "title": "Dune",
            "available": true
        });

        importer.import_value(&payload).unwrap();
        assert!(!importer.metadata().is_empty());

        importer.clear();

        assert!(importer.data().is_empty());
        assert!(importer.metadata().is_empty());

        importer.import_value(&payload).unwrap();
        let metadata = importer.metadata();
        let title_attr = Attribute::<Handle<Blake3, LongString>>::from_field("title").id();
        assert_attribute_metadata::<Handle<Blake3, LongString>>(&metadata, title_attr, "title");
    }

    #[test]
    fn deterministic_importer_reimports_stably() {
        let mut importer = make_deterministic_importer();
        let payload = serde_json::json!({
            "title": "Dune",
            "pages": 412,
            "available": true,
            "tags": ["scifi", "classic"],
            "author": {
                "first": "Frank",
                "last": "Herbert"
            }
        });

        importer.import_value(&payload).unwrap();
        let first = importer.data().clone();

        importer.import_value(&payload).unwrap();
        let second = importer.data().clone();

        assert_eq!(first, second);
    }

    #[test]
    fn deterministic_importer_ignores_field_order() {
        let mut importer = make_deterministic_importer();
        let payload_a = serde_json::json!({
            "title": "Dune",
            "tags": ["classic", "scifi"],
            "author": {
                "last": "Herbert",
                "first": "Frank"
            }
        });
        let payload_b = serde_json::json!({
            "author": {
                "first": "Frank",
                "last": "Herbert"
            },
            "title": "Dune",
            "tags": ["scifi", "classic"]
        });

        importer.import_value(&payload_a).unwrap();
        let first = importer.data().clone();

        importer.clear_data();

        importer.import_value(&payload_b).unwrap();
        let second = importer.data().clone();

        assert_eq!(first, second);
    }

    #[test]
    fn deterministic_importer_handles_top_level_arrays() {
        let mut importer = make_deterministic_importer();
        let payload = serde_json::json!([
            { "title": "Dune" },
            { "title": "Dune Messiah" }
        ]);

        importer.import_value(&payload).unwrap();
        let first = importer.data().clone();
        let metadata = importer.metadata();

        let title_attr = Attribute::<Handle<Blake3, LongString>>::from_field("title").id();
        let mut by_root = std::collections::HashMap::new();
        for trible in &first {
            assert_eq!(trible.a(), &title_attr);
            by_root.insert(*trible.e(), trible.v::<Handle<Blake3, LongString>>().raw);
        }

        assert_eq!(by_root.len(), 2);
        let observed: std::collections::BTreeSet<_> = by_root.values().copied().collect();
        let expected: std::collections::BTreeSet<_> = ["Dune", "Dune Messiah"]
            .into_iter()
            .map(|title| {
                ToBlob::<LongString>::to_blob(title)
                    .get_handle::<Blake3>()
                    .raw
            })
            .collect();
        assert_eq!(observed, expected);
        assert_attribute_metadata::<Handle<Blake3, LongString>>(&metadata, title_attr, "title");

        importer.clear_data();
        importer.import_value(&payload).unwrap();
        let second = importer.data().clone();

        assert_eq!(first, second);
        for trible in &second {
            assert!(by_root.contains_key(trible.e()));
        }
    }

    #[test]
    fn deterministic_clear_resets_cached_attributes() {
        let mut importer = make_deterministic_importer();
        let payload = serde_json::json!({
            "title": "Dune",
            "available": true
        });

        importer.import_value(&payload).unwrap();
        assert!(!importer.metadata().is_empty());

        importer.clear();

        assert!(importer.data().is_empty());
        assert!(importer.metadata().is_empty());

        importer.import_value(&payload).unwrap();
        let metadata = importer.metadata();
        let title_attr = Attribute::<Handle<Blake3, LongString>>::from_field("title").id();
        assert_attribute_metadata::<Handle<Blake3, LongString>>(&metadata, title_attr, "title");
    }

    #[test]
    fn deterministic_importer_rejects_primitive_roots() {
        let mut importer = make_deterministic_importer();
        let payload = serde_json::json!(42);
        let err = importer.import_value(&payload).unwrap_err();
        match err {
            JsonImportError::PrimitiveRoot => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
