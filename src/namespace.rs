pub type Id = [u8; 16];
pub type Value = [u8; 32];
pub type Blob = Vec<u8>;

pub trait Factory {
    fn factory() -> Self;
}
/*        lovedBy: UFOID => inv "328edd7583de04e2bedd6bd4fd50e651",
 */

macro_rules! entities_inner {
    (@triple ($set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, $Value:expr)) => {
        $set.add(&crate::trible::Trible::new(
            $EntityId,
            { use $Namespace as base; base::ids::$FieldName },
            { use $Namespace as base; let v: base::types::$FieldName = $Value; v}))
    };
    (@entity ($set:ident, $Namespace:path, {$EntityId:ident @ $($FieldName:ident : $Value:expr),*})) => {
        $(entities_inner!(@triple ($set, $Namespace, $EntityId, $FieldName, $Value));)*
    };
    (@entity ($set:ident, $Namespace:path, {$($FieldName:ident : $Value:expr),*})) => {
        {
            {
                let id = { use $Namespace as base; <base::Id as crate::namespace::Factory>::factory() };
                $(entities_inner!(@triple ($set, $Namespace, id, $FieldName, $Value));)*
            }
        }
    };
    ($Namespace:path, ($($Var:ident),*), [$($Entity:tt),*]) => {
        {
            let mut set = crate::tribleset::TribleSet::new();
            $(let $Var = { use $Namespace as base; <base::Id as crate::namespace::Factory>::factory() };)*
            $(entities_inner!(@entity (set, $Namespace, $Entity));)*
            set
        }
    };
}
pub(crate) use entities_inner;

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
    ($visibility:vis namespace $mod_name:ident {@ $IdType:ty; $($FieldName:ident: $FieldId:literal as $FieldType:ty;)*}) => {
        $visibility mod $mod_name {
            pub type Id = $IdType;
            pub mod ids {
                #![allow(non_upper_case_globals)]
                $(pub const $FieldName:$IdType = <$IdType>::raw(hex_literal::hex!($FieldId));)*
            }
            pub mod types {
                #![allow(non_camel_case_types)]
                $(pub type $FieldName = $FieldType;)*
            }

            pub(crate) use entities_inner;

            #[macro_export]
            macro_rules! entities {
                ($vars:tt, $entities: tt) => {
                    entities_inner!($mod_name, $vars, $entities)
                };
            }

            pub use entities;
        }
    };
}

pub(crate) use NS;

NS! {
    pub namespace knights {
        @ crate::types::ufoid::UFOID;
        loves: "328edd7583de04e2bedd6bd4fd50e651" as crate::types::ufoid::UFOID;
        name: "328147856cc1984f0806dbb824d2b4cb" as crate::types::shortstring::ShortString;
        title: "328f2c33d2fdd675e733388770b2d6c4" as crate::types::shortstring::ShortString;
    }
}

#[cfg(test)]
mod tests {
    use super::knights;
    use std::convert::TryInto;

    #[test]
    fn ns_entities() {
        println!(
            "{:?}",
            knights::entities!((romeo, juliet),
            [{juliet @
                name: "Juliet".try_into().unwrap(),
                loves: romeo,
                title: "Maiden".try_into().unwrap()
            },
            {romeo @
                name: "Romeo".try_into().unwrap(),
                loves: juliet,
                title: "Prince".try_into().unwrap()
            },
            {
                name: "Angelica".try_into().unwrap(),
                title: "Nurse".try_into().unwrap()
            }])
        );
    }
}
