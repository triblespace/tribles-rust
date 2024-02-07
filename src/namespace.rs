//! Namespaces give semantic meaning to the raw binary data stored in
//! [crate::TribleSet]s and [crate::BlobSet]s and provide a mapping to human readable
//! names and language types.
//!
//! Note that the namespace system (and in extend data model) presented here
//! is just one of potentially many ways to create and query trible and blob data,
//! and you are encouraged to port or invent the data definition and query languages
//! that fit your personal needs and taste, e.g. GraphQL, SQL, Cypher, SPARQL and friend.
//!
//! Great care has been taken to design the system in a way that data described
//! in different data definition languages can be merged, and more importanly
//! that multiple query languages can be cooperatively used in a single query.

#[doc(hidden)]
#[macro_export]
macro_rules! entities_inner {
    (@triple ($set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, $Value:expr)) => {
        $set.insert(&$crate::trible::Trible::new(
            $EntityId,
            { use $Namespace as ns; ns::ids::$FieldName },
            { use $Namespace as ns; let v: ns::types::$FieldName = $Value; v}))
    };
    (@entity ($set:ident, $Namespace:path, {$EntityId:ident @ $($FieldName:ident : $Value:expr),* $(,)?})) => {
        $(entities_inner!(@triple ($set, $Namespace, $EntityId, $FieldName, $Value));)*
    };
    (@entity ($set:ident, $Namespace:path, {$($FieldName:ident : $Value:expr),* $(,)?})) => {
        {
            {
                let id = { use $Namespace as ns; <ns::Id as $crate::types::Idlike>::factory() };
                $(entities_inner!(@triple ($set, $Namespace, id, $FieldName, $Value));)*
            }
        }
    };
    ($Namespace:path, ($($Var:ident),*), [$($Entity:tt),*], $set:ident) => {
        {
            $(let $Var = { use $Namespace as ns; <ns::Id as $crate::types::Idlike>::factory() };)*
            $(entities_inner!(@entity ($set, $Namespace, $Entity));)*
            $set
        }
    };
    ($Namespace:path, ($($Var:ident),*), [$($Entity:tt),*]) => {
        {
            let mut set = $crate::TribleSet::new();
            entities_inner!($Namespace, ($($Var),*), [$($Entity),*], set)
        }
    };
}
pub use entities_inner;

#[doc(hidden)]
#[macro_export]
macro_rules! pattern_inner {
    (@triple ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, ($Value:expr))) => {
        {
            use $crate::query::TriblePattern;
            let a_var: $crate::query::Variable<ns::Id> = $ctx.next_variable();
            let v_var: $crate::query::Variable<ns::types::$FieldName> = $ctx.next_variable();
            $constraints.push({ use $Namespace as ns; Box::new(a_var.is(ns::ids::$FieldName)) });
            $constraints.push({ use $Namespace as ns; let v: ns::types::$FieldName = $Value; Box::new(v_var.is(v))});
            $constraints.push(Box::new($set.pattern($EntityId, a_var, v_var)));
        }

    };
    (@triple ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, $Value:expr)) => {
        {
            use $crate::query::TriblePattern;
            use $Namespace as ns;
            let a_var: $crate::query::Variable<ns::Id> = $ctx.next_variable();
            let v_var: $crate::query::Variable<ns::types::$FieldName> = $Value;
            $constraints.push(Box::new(a_var.is(ns::ids::$FieldName)));
            $constraints.push(Box::new($set.pattern($EntityId, a_var, v_var)));
        }

    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {($EntityId:expr) @ $($FieldName:ident : $Value:tt),* $(,)?})) => {
        {
            use $Namespace as ns;
            let e_var: $crate::query::Variable<ns::Id> = $ctx.next_variable();
            $constraints.push({ use $Namespace as ns; let e: ns::Id = $EntityId; Box::new(e_var.is(e))});
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {$EntityId:ident @ $($FieldName:ident : $Value:tt),* $(,)?})) => {
        {
            use $Namespace as ns;
            let e_var: $crate::query::Variable<ns::Id> = $EntityId;
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {$($FieldName:ident : $Value:tt),*})) => {
        {
            use $Namespace as ns;
            let e_var: $crate::query::Variable<ns::Id> = $ctx.next_variable();
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

/// Define a rust module to represent a namespace.
/// The module additionally defines `entities!` and `pattern!` macros.
///
/// The `entities!` macro can be used to conveniently create triblesets
/// containing entities conforming to the namespace.
///
/// The `pattern!` macro can be used to query datastructures implementing
/// the [crate::query::TriblePattern] trait.
///
/// A namespace defined like this
/// ```
/// use tribles::NS;
///
/// NS! {
///     pub namespace namespace_name {
///         @ tribles::types::syntactic::UFOID;
///         attr_name: "FF00FF00FF00FF00FF00FF00FF00FF00" as tribles::types::syntactic::UFOID;
///         attr_name2: "BBAABBAABBAABBAABBAABBAABBAABBAA" as tribles::types::syntactic::ShortString;
///     }
/// }
/// ```
///
/// will be translated into a module with the following structure
///
/// ```
/// mod namespace_name {
///   pub use tribles::types::syntactic::UFOID as id;
///   pub mod ids {
///       use hex_literal::hex;
///       pub const attr_name: tribles::types::syntactic::UFOID  = tribles::types::syntactic::UFOID::raw(hex!("FF00FF00FF00FF00FF00FF00FF00FF00"));
///       pub const attr_name2: tribles::types::syntactic::UFOID  = tribles::types::syntactic::UFOID::raw(hex!("BBAABBAABBAABBAABBAABBAABBAABBAA"));
///   }
///   pub mod types {
///       pub use tribles::types::syntactic::UFOID as attr_name;
///       pub use tribles::types::syntactic::ShortString as attr_name2;
///   }
/// }
/// ```
///
/// this allows you to access attribute ids and types via their human readable names, e.g.
/// `namespace_name::ids::attrName` and `namespace_name::types::attrName`.
#[macro_export]
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

            #[allow(unused)]
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

            #[allow(unused)]
            pub(crate) use entities;

            #[allow(unused)]
            macro_rules! pattern {
                ($ctx:ident, $set:expr, $pattern: tt) => {
                    {
                        use $crate::namespace::pattern_inner;
                        pattern_inner!($mod_name, $ctx, $set, $pattern)
                    }
                };
            }

            #[allow(unused)]
            pub(crate) use pattern;
        }
    };
}

pub use NS;

#[cfg(test)]
mod tests {
    use fake::{faker::name::raw::Name, locales::EN, Fake};

    use crate::{patch::init, query::find, TribleSet};

    use std::convert::TryInto;

    NS! {
        pub namespace knights {
            @ crate::types::syntactic::UFOID;
            loves: "328edd7583de04e2bedd6bd4fd50e651" as crate::types::syntactic::UFOID;
            name: "328147856cc1984f0806dbb824d2b4cb" as crate::types::syntactic::ShortString;
            title: "328f2c33d2fdd675e733388770b2d6c4" as crate::types::syntactic::ShortString;
        }
    }

    #[test]
    fn ns_entities() {
        init();
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
        init();

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
        let r: Vec<_> = find!(
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
        assert_eq!(vec![Ok((juliet, "Juliet".try_into().unwrap(),))], r);
    }

    #[test]
    fn ns_pattern_large() {
        init();

        let mut kb = TribleSet::new();
        (0..10000).for_each(|_| {
            kb.union(&knights::entities!((lover_a, lover_b),
            [{lover_a @
                name: Name(EN).fake::<String>().try_into().unwrap(),
                loves: lover_b
            },
            {lover_b @
                name: Name(EN).fake::<String>().try_into().unwrap(),
                loves: lover_a
            }]));
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

        kb.union(&data_kb);

        let r: Vec<_> = find!(
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

        assert_eq!(vec![Ok((juliet, "Juliet".try_into().unwrap(),))], r);
    }
}
