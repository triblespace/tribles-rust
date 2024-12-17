use crate::{id::Id, id_hex, NS};

pub const ATTR_VALUE_SCHEMA: Id = id_hex!("213F89E3F49628A105B3830BD3A6612C");
pub const ATTR_BLOB_SCHEMA: Id = id_hex!("02FAF947325161918C6D2E7D9DBA3485");
pub const ATTR_NAME: Id = id_hex!("2E26F8BA886495A8DF04ACF0ED3ACBD4");

NS! {
    pub namespace metadata {
        "2E26F8BA886495A8DF04ACF0ED3ACBD4" as attr_name: crate::prelude::valueschemas::ShortString;
        "213F89E3F49628A105B3830BD3A6612C" as attr_value_schema: crate::prelude::valueschemas::GenId;
        "02FAF947325161918C6D2E7D9DBA3485" as attr_blob_schema: crate::prelude::valueschemas::GenId;
    }
}
