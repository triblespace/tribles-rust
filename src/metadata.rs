use crate::{id::Id, id_hex, NS};

pub const ATTR_ATTR_VALUE_SCHEMA: Id = id_hex!("213F89E3F49628A105B3830BD3A6612C");
pub const ATTR_ATTR_BLOB_SCHEMA: Id = id_hex!("02FAF947325161918C6D2E7D9DBA3485");
pub const ATTR_LABEL: Id = id_hex!("2E26F8BA886495A8DF04ACF0ED3ACBD4");

NS! {
    pub namespace metadata {
        "213F89E3F49628A105B3830BD3A6612C" as attr_value_schema: crate::prelude::valueschemas::GenId;
        "02FAF947325161918C6D2E7D9DBA3485" as attr_blob_schema: crate::prelude::valueschemas::GenId;
        "2E26F8BA886495A8DF04ACF0ED3ACBD4" as label: crate::prelude::valueschemas::ShortString;
    }
}
