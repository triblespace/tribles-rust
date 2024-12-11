use crate::{id::Id, NS};

pub const ATTR_ATTR_SCHEMA: Id =
    Id::new(hex_literal::hex!("213F89E3F49628A105B3830BD3A6612C")).unwrap();
pub const ATTR_LABEL: Id = Id::new(hex_literal::hex!("2E26F8BA886495A8DF04ACF0ED3ACBD4")).unwrap();

NS! {
    pub namespace metadata {
        "213F89E3F49628A105B3830BD3A6612C" as attr_schema: crate::prelude::valueschemas::GenId;
        "2E26F8BA886495A8DF04ACF0ED3ACBD4" as label: crate::prelude::valueschemas::ShortString;
    }
}
