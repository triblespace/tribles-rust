//use crate::query::{query, Query, Variable};

pub type Id = [u8; 16];
pub type Value = [u8; 32];
pub type Blob = Vec<u8>;

pub trait Factory {
    fn factory() -> Self;
}
/*        lovedBy: UFOID => inv "328edd7583de04e2bedd6bd4fd50e651",
 */

#[macro_export]
macro_rules! entities_inner {
    (@triple ($set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, $Value:expr)) => {
        $set.add(&$crate::trible::Trible::new(
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
                let id = { use $Namespace as base; <base::Id as $crate::namespace::Factory>::factory() };
                $(entities_inner!(@triple ($set, $Namespace, id, $FieldName, $Value));)*
            }
        }
    };
    ($Namespace:path, ($($Var:ident),*), [$($Entity:tt),*], $set:ident) => {
        {
            $(let $Var = { use $Namespace as base; <base::Id as $crate::namespace::Factory>::factory() };)*
            $(entities_inner!(@entity ($set, $Namespace, $Entity));)*
            $set
        }
    };
    ($Namespace:path, ($($Var:ident),*), [$($Entity:tt),*]) => {
        {
            let mut set = $crate::tribleset::patchtribleset::PATCHTribleSet::new();
            entities_inner!($Namespace, ($($Var),*), [$($Entity),*], set)
        }
    };
}
pub use entities_inner;

#[macro_export]
macro_rules! pattern_inner {
    (@triple ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, ($Value:expr))) => {
        {
            use $crate::tribleset::TribleSet;
            let a_var: $crate::query::Variable<base::Id> = $ctx.next_variable();
            let v_var: $crate::query::Variable<base::types::$FieldName> = $ctx.next_variable();
            $constraints.push({ use $Namespace as base; Box::new(a_var.is(base::ids::$FieldName)) });
            $constraints.push({ use $Namespace as base; let v: base::types::$FieldName = $Value; Box::new(v_var.is(v))});
            $constraints.push(Box::new($set.pattern($EntityId, a_var, v_var)));
        }

    };
    (@triple ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, $Value:expr)) => {
        {
            use $crate::tribleset::TribleSet;
            use $Namespace as base;
            let a_var: $crate::query::Variable<base::Id> = $ctx.next_variable();
            let v_var: $crate::query::Variable<base::types::$FieldName> = $Value;
            $constraints.push(Box::new(a_var.is(base::ids::$FieldName)));
            $constraints.push(Box::new($set.pattern($EntityId, a_var, v_var)));
        }

    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {($EntityId:expr) @ $($FieldName:ident : $Value:tt),*})) => {
        {
            use $Namespace as base;
            let e_var: $crate::query::Variable<base::Id> = $ctx.next_variable();
            $constraints.push({ use $Namespace as base; let e: base::Id = $EntityId; Box::new(e_var.is(e))});
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {$EntityId:ident @ $($FieldName:ident : $Value:tt),*})) => {
        {
            use $Namespace as base;
            let e_var: $crate::query::Variable<base::Id> = $EntityId;
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {$($FieldName:ident : $Value:tt),*})) => {
        {
            use $Namespace as base;
            let e_var: $crate::query::Variable<base::Id> = $ctx.next_variable();
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };
    ($Namespace:path, $ctx:ident, $set:expr, [$($Entity:tt),*]) => {
        {
            let set = &($set);
            let mut constraints: Vec<Box<dyn $crate::query::Constraint>> = vec!();
            $(pattern_inner!(@entity (constraints, $ctx, set, $Namespace, $Entity));)*
            $crate::query::IntersectionConstraint::new(constraints)
        }
    };
}

pub use pattern_inner;

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

            #[macro_export]
            macro_rules! entities {
                ($vars:tt, $entities: tt, $set: ident) => {
                    {
                        use $crate::namespace::entities_inner;
                        entities_inner!($mod_name, $vars, $entities, $set)
                    }
                };
                ($vars:tt, $entities: tt) => {
                    {
                        use $crate::namespace::entities_inner;
                        entities_inner!($mod_name, $vars, $entities)
                    }
                };
            }

            pub use entities;

            #[macro_export]
            macro_rules! pattern {
                ($ctx:ident, $set:expr, $pattern: tt) => {
                    {
                        use $crate::namespace::pattern_inner;
                        pattern_inner!($mod_name, $ctx, $set, $pattern)
                    }
                };
            }

            pub use pattern;
        }
    };
}

pub(crate) use NS;

NS! {
    pub namespace knights {
        @ crate::types::syntactic::UFOID;
        loves: "328edd7583de04e2bedd6bd4fd50e651" as crate::types::syntactic::UFOID;
        name: "328147856cc1984f0806dbb824d2b4cb" as crate::types::syntactic::ShortString;
        title: "328f2c33d2fdd675e733388770b2d6c4" as crate::types::syntactic::ShortString;
    }
}

#[cfg(test)]
mod tests {
    use fake::{faker::name::raw::Name, locales::EN, Fake};

    use crate::{query, tribleset::patchtribleset::PATCHTribleSet};

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

    #[test]
    fn ns_pattern() {
        let juliet = knights::Id::new();
        let kb = knights::entities!((romeo),
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
        }]);
        let r: Vec<_> = query!(
            ctx,
            (juliet, name),
            knights::pattern!(ctx, kb, [
            {name: ("Romeo".try_into().unwrap()),
             loves: juliet},
            {juliet @
                name: name
            }])
        )
        .collect();
        assert_eq!(vec![(juliet, "Juliet".try_into().unwrap(),)], r);
    }

    #[test]
    fn ns_pattern_large() {
        let mut background_kb = PATCHTribleSet::new();
        (0..10000).for_each(|_| {
            let kb = &mut background_kb;
            knights::entities!((lover_a, lover_b),
                [{lover_a @
                    name: Name(EN).fake::<String>().try_into().unwrap(),
                    loves: lover_b
                },
                {lover_b @
                    name: Name(EN).fake::<String>().try_into().unwrap(),
                    loves: lover_a
                }], kb);
        });

        let juliet = knights::Id::new();
        let data_kb = knights::entities!((romeo),
        [{juliet @
            name: "Juliet".try_into().unwrap(),
            loves: romeo
        },
        {romeo @
            name: "Romeo".try_into().unwrap(),
            loves: juliet
        }]);

        let kb: PATCHTribleSet =
            PATCHTribleSet::union([background_kb.clone(), data_kb.clone()].iter());

        let r: Vec<_> = query!(
            ctx,
            (juliet, name),
            knights::pattern!(ctx, kb, [
            {name: ("Romeo".try_into().unwrap()),
             loves: juliet},
            {juliet @
                name: name
            }])
        )
        .collect();

        assert_eq!(vec![(juliet, "Juliet".try_into().unwrap(),)], r);
    }
}
