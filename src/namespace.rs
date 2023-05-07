use blake2::digest::consts::U32;
use blake2::{Blake2b, Digest};

/*
macro_rules! outer {
    ($mod_name:ident) => {
        pub mod $mod_name {
            #[macro_export]
            macro_rules! inner {
                () => {
                    1
                };
            }
        }
    };
}

outer!(some_mod);
const X: usize = some_mod::entity!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_namespace() {
        some_ns::entity(1);
    }
}
*/

pub trait Id {
    fn decode(data: [u8; 16]) -> Self;
    fn encode(id: Self) -> [u8; 16];
    fn factory() -> Self;
}

pub trait Value {
    fn decode(data: [u8; 32], blob: fn() -> Option<Vec<u8>>) -> Self;
    fn encode(value: Self) -> ([u8; 32], Option<Vec<u8>>);
}

impl Value for String {
    fn decode(data: [u8; 32], blob: fn() -> Option<Vec<u8>>) -> Self {
        String::from_utf8(blob().unwrap()).unwrap()
    }
    fn encode(value: Self) -> ([u8; 32], Option<Vec<u8>>) {
        let bytes = value.into_bytes();
        let data: [u8; 32] = Blake2b::<U32>::digest(&bytes).into();
        (data, Some(bytes))
    }
}

/*
mod knights {
    pub use crate::ufoid::UFOID as id;
    pub mod ids {
        use hex_literal::hex;
        pub const name: crate::ufoid::UFOID  = crate::ufoid::UFOID::raw(hex!("328147856cc1984f0806dbb824d2b4cb"));
        pub const loves: crate::ufoid::UFOID  = crate::ufoid::UFOID::raw(hex!("328edd7583de04e2bedd6bd4fd50e651"));
        pub const title: crate::ufoid::UFOID  = crate::ufoid::UFOID::raw(hex!("328f2c33d2fdd675e733388770b2d6c4"));
    }
    pub mod types {
        pub use crate::ufoid::UFOID as loves;
        pub use std::string::String as name;
        pub use std::string::String as title;
    }
}
*/

macro_rules! NS {
    ($mod_name:ident {$IdType:ty, $($FieldName:ident: $FieldType:ty => $FieldId:expr;)*}) => {
        pub mod $mod_name {
            pub type Id = $IdType;
            pub mod ids {
                #![allow(non_upper_case_globals)]
                $(pub const $FieldName:$IdType = $FieldId;)*
            }
            pub mod types {
                #![allow(non_camel_case_types)]
                $(pub type $FieldName = $FieldType;)*
            }
        }
    };
}

pub(crate) use NS;

NS! {
    knights {crate::ufoid::UFOID,
        loves: crate::ufoid::UFOID => crate::ufoid::UFOID::raw(hex_literal::hex!("328edd7583de04e2bedd6bd4fd50e651"));
        name: String => crate::ufoid::UFOID::raw(hex_literal::hex!("328147856cc1984f0806dbb824d2b4cb"));
        title: String => crate::ufoid::UFOID::raw(hex_literal::hex!("328f2c33d2fdd675e733388770b2d6c4"));
    }
}
/*        lovedBy: UFOID => inv "328edd7583de04e2bedd6bd4fd50e651",
 */

macro_rules! entity {
    ($Namespace:path, {$EntityId:ident, $($FieldName:ident : $Value:expr),*}) => {
        {
            [$(crate::trible::Trible::new($EntityId,
                { use $Namespace as base; base::ids::$FieldName },
                { use $Namespace as base; base::types::$FieldName::from($Value) })),*]
        }
    };
    ($Namespace:path, {$($FieldName:ident : $Value:expr),*}) => {
        {
            {let id = { use $Namespace as base; <base::Id as crate::namespace::Id>::factory() };
                [$(crate::trible::Trible::new(id,
                    { use $Namespace as base; base::ids::$FieldName },
                    { use $Namespace as base; base::types::$FieldName::from($Value) })),*]
            }
        }
    };
}
pub(crate) use entity;

macro_rules! entities {
    ($Namespace:path, ($($Var:ident),*), [$($Entity:tt),*]) => {
        {
            $(let $Var = { use $Namespace as base; <base::Id as crate::namespace::Id>::factory() };)*
            [$(entity!($Namespace, $Entity)),*]
        }
    };
}
pub(crate) use entities;

#[cfg(test)]
mod tests {
    use super::entities;
    use super::entity;
    use super::knights;

    #[test]
    fn ns_entity() {
        let romeo = knights::Id::new();
        let juliet = knights::Id::new();
        println!(
            "{:?}",
            entity!(knights, {romeo,
                name: "Romeo",
                loves: juliet,
                title: "Prince"
            })
        );
    }

    #[test]
    fn ns_entity_noid() {
        let juliet = knights::Id::new();
        println!(
            "{:?}",
            entity!(knights, {
                name: "Romeo",
                loves: juliet,
                title: "Prince"
            })
        );
    }

    #[test]
    fn ns_entities() {
        println!(
            "{:?}",
            entities!(knights, (romeo, juliet),
            [{juliet,
                name: "Juliet",
                loves: romeo,
                title: "Maiden"
            },
            {romeo,
                name: "Romeo",
                loves: juliet,
                title: "Prince"
            }])
        );
    }
}
