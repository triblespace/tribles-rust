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

type Value = [u8; 32];

pub fn encode_id(value: Value) -> Value {
    value
}

pub fn encode_string(value: String) -> Value {
    [1; 32]
}

pub fn factory() -> Value {
    [0; 32]
}

mod knightsNS {
    pub mod id {
        pub use crate::namespace::factory as factory;
    }
    pub mod ids {
        use hex_literal::hex;
        pub const name: [u8; 16] = hex!("328147856cc1984f0806dbb824d2b4cb");
        pub const loves: [u8; 16] = hex!("328edd7583de04e2bedd6bd4fd50e651");
        pub const title: [u8; 16] = hex!("328f2c33d2fdd675e733388770b2d6c4");
    }
    pub mod types {
        pub use crate::namespace::encode_id as loves;
        pub use crate::namespace::encode_string as name;
        pub use crate::namespace::encode_string as title;
    }
}
/*
NS!{
    knights {
        @: UFOID,
        name: String @ "328147856cc1984f0806dbb824d2b4cb",
        loves @ "328edd7583de04e2bedd6bd4fd50e651",
        lovedBy @ inv "328edd7583de04e2bedd6bd4fd50e651",
    }
}
 
macro_rules! NS {
    ({$($FieldName:ident : $Value:expr),*}) => {
        {
            [$(($EntityId,
                { use $Namespace as base; base::ids::$FieldName },
                { use $Namespace as base; base::encoders::$FieldName($Value.into()) })),*]
        }
    };
}
pub(crate) use NS;
*/

macro_rules! entity {
    ($Namespace:path, {@:$EntityId:expr, $($FieldName:ident : $Value:expr),*}) => {
        {
            [$(($EntityId,
                { use $Namespace as base; base::ids::$FieldName },
                { use $Namespace as base; base::encoders::$FieldName($Value.into()) })),*]
        }
    };
    ($Namespace:path, {$($FieldName:ident : $Value:expr),*}) => {
        {
            {let id = { use $Namespace as base; base::id::factory() };
                [$((id,
                    { use $Namespace as base; base::ids::$FieldName },
                    { use $Namespace as base; base::encoders::$FieldName($Value.into()) })),*]
            }
        }
    };
}
pub(crate) use entity;

macro_rules! entities {
    ($Namespace:path, ($($Var:ident),*), [$($Entity:tt),*]) => {
        {
            $(let $Var = { use $Namespace as base; base::id::factory() };)*
            [$(entity!($Namespace, $Entity)),*]
        }
    };
}
pub(crate) use entities;

#[cfg(test)]
mod tests {
    use super::entities;
    use super::entity;
    use super::knightsNS;

    #[test]
    fn ns_entity() {
        println!(
            "{:?}",
            entity!(knightsNS, {
                @:"32d86c15fa6818b8335d15ff39281ec1",
                name: "Romeo",
                loves: [0;32],
                title: "Prince"
            })
        );
    }

    #[test]
    fn ns_entities() {
        println!(
            "{:?}",
            entities!(knightsNS, (romeo, juliet),
                [{
                @:romeo,
                name: "Romeo",
                loves: juliet,
                title: "Prince"
            },
            {
                @:juliet,
                name: "Juliet",
                loves: romeo,
                title: "Maiden"
            }])
        );
    }
}
