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

pub type Id = [u8; 16];
pub type Value = [u8; 32];
pub type Blob = Vec<u8>;

pub trait Factory {
    fn factory() -> Self;
}

/*
mod knights {
    pub use crate::types::ufoid::UFOID as id;
    pub mod ids {
        use hex_literal::hex;
        pub const name: crate::types::ufoid::UFOID  = crate::types::ufoid::UFOID::raw(hex!("328147856cc1984f0806dbb824d2b4cb"));
        pub const loves: crate::types::ufoid::UFOID  = crate::types::ufoid::UFOID::raw(hex!("328edd7583de04e2bedd6bd4fd50e651"));
        pub const title: crate::types::ufoid::UFOID  = crate::types::ufoid::UFOID::raw(hex!("328f2c33d2fdd675e733388770b2d6c4"));
    }
    pub mod types {
        pub use crate::types::ufoid::UFOID as loves;
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
    knights {crate::types::ufoid::UFOID,
        loves: crate::types::ufoid::UFOID => crate::types::ufoid::UFOID::raw(hex_literal::hex!("328edd7583de04e2bedd6bd4fd50e651"));
        name: crate::types::shortstring::ShortString => crate::types::ufoid::UFOID::raw(hex_literal::hex!("328147856cc1984f0806dbb824d2b4cb"));
        title: crate::types::shortstring::ShortString => crate::types::ufoid::UFOID::raw(hex_literal::hex!("328f2c33d2fdd675e733388770b2d6c4"));
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
            {let id = { use $Namespace as base; <base::Id as crate::namespace::Factory>::factory() };
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
            $(let $Var = { use $Namespace as base; <base::Id as crate::namespace::Factory>::factory() };)*
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
    use crate::types::shortstring::ShortString;

    #[test]
    fn ns_entity() {
        let romeo = knights::Id::new();
        let juliet = knights::Id::new();
        println!(
            "{:?}",
            entity!(knights, {romeo,
                name: ShortString::new("Romeo".to_string()).unwrap(),
                loves: juliet,
                title: ShortString::new("Prince".to_string()).unwrap()
            })
        );
    }

    #[test]
    fn ns_entity_noid() {
        let juliet = knights::Id::new();
        println!(
            "{:?}",
            entity!(knights, {
                name: ShortString::new("Romeo".to_string()).unwrap(),
                loves: juliet,
                title: ShortString::new("Prince".to_string()).unwrap()
            })
        );
    }

    #[test]
    fn ns_entities() {
        println!(
            "{:?}",
            entities!(knights, (romeo, juliet),
            [{juliet,
                name: ShortString::new("Juliet".to_string()).unwrap(),
                loves: romeo,
                title: ShortString::new("Maiden".to_string()).unwrap()
            },
            {romeo,
                name: ShortString::new("Romeo".to_string()).unwrap(),
                loves: juliet,
                title: ShortString::new("Prince".to_string()).unwrap()
            }])
        );
    }
}
