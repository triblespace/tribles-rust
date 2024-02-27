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
macro_rules! entity_inner {
    ($Namespace:path, {$($FieldName:ident : $Value:expr),* $(,)?}) => {
        {
            {
                use $Namespace as ns;
                let mut set = $crate::TribleSet::new();
                let id = $crate::idgen();
                $({let v: ns::types::$FieldName = $Value;
                    set.insert(&$crate::trible::Trible::new(
                    id,
                    ns::ids::$FieldName,
                    v));};)*
                set
            }
        }
    };
    ($Namespace:path, $EntityId:expr, {$($FieldName:ident : $Value:expr),* $(,)?}) => {
        {
            use $Namespace as ns;
            let mut set = $crate::TribleSet::new();
            $({ let v: ns::types::$FieldName = $Value;
                set.insert(&$crate::trible::Trible::new(
                $EntityId,
                ns::ids::$FieldName,
                v));})*
            set
        }
    };
}

pub use entity_inner;

#[doc(hidden)]
#[macro_export]
macro_rules! pattern_inner {
    (@triple ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, ($Value:expr))) => {
        {
            use $crate::query::TriblePattern;
            use $Namespace as ns;
            let a_var: $crate::query::Variable<$crate::Id> = $ctx.next_variable();
            let v_var: $crate::query::Variable<ns::types::$FieldName> = $ctx.next_variable();
            let v: ns::types::$FieldName = $Value;
            $constraints.push(Box::new(a_var.is(ns::ids::$FieldName)));
            $constraints.push(Box::new(v_var.is(v)));
            $constraints.push(Box::new($set.pattern($EntityId, a_var, v_var)));
        }

    };
    (@triple ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, $Value:expr)) => {
        {
            use $crate::query::TriblePattern;
            use $Namespace as ns;
            let a_var: $crate::query::Variable<$crate::Id> = $ctx.next_variable();
            let v_var: $crate::query::Variable<ns::types::$FieldName> = $Value;
            $constraints.push(Box::new(a_var.is(ns::ids::$FieldName)));
            $constraints.push(Box::new($set.pattern($EntityId, a_var, v_var)));
        }

    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {($EntityId:expr) @ $($FieldName:ident : $Value:tt),* $(,)?})) => {
        {
            let e_var: $crate::query::Variable<$crate::Id> = $ctx.next_variable();
            $constraints.push({ let e: $crate::Id = $EntityId; Box::new(e_var.is(e))});
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {$EntityId:ident @ $($FieldName:ident : $Value:tt),* $(,)?})) => {
        {
            let e_var: $crate::query::Variable<$crate::Id> = $EntityId;
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {$($FieldName:ident : $Value:tt),*})) => {
        {
            let e_var: $crate::query::Variable<$crate::Id> = $ctx.next_variable();
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

pub use hex_literal;

/// Define a rust module to represent a namespace.
/// The module additionally defines `entity!` and `pattern!` macros.
///
/// The `entity!` macro can be used to conveniently create triblesets
/// containing an entity conforming to the namespace.
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
///         attr_name: "FF00FF00FF00FF00FF00FF00FF00FF00" as tribles::Id;
///         attr_name2: "BBAABBAABBAABBAABBAABBAABBAABBAA" as tribles::types::ShortString;
///     }
/// }
/// ```
///
/// will be translated into a module with the following structure
///
/// ```
/// mod namespace_name {
///   pub mod ids {
///       use hex_literal::hex;
///       pub const attr_name: tribles::Id  = hex!("FF00FF00FF00FF00FF00FF00FF00FF00");
///       pub const attr_name2: tribles::Id  = hex!("BBAABBAABBAABBAABBAABBAABBAABBAA");
///   }
///   pub mod types {
///       pub use tribles::Id as attr_name;
///       pub use tribles::types::ShortString as attr_name2;
///   }
/// }
/// ```
///
/// this allows you to access attribute ids and types via their human readable names, e.g.
/// `namespace_name::ids::attrName` and `namespace_name::types::attrName`.
#[macro_export]
macro_rules! NS {
    ($visibility:vis namespace $mod_name:ident {$($FieldName:ident: $FieldId:literal as $FieldType:ty;)*}) => {
        $visibility mod $mod_name {
            pub mod ids {
                #![allow(non_upper_case_globals)]
                $(pub const $FieldName:$crate::Id = $crate::namespace::hex_literal::hex!($FieldId);)*
            }
            pub mod types {
                #![allow(non_camel_case_types)]
                $(pub type $FieldName = $FieldType;)*
            }
            
            #[allow(unused)]
            macro_rules! entity {
                ($entity:tt) => {
                    {
                        use $crate::namespace::entity_inner;
                        entity_inner!($mod_name, $entity)
                    }
                };
                ($entity_id:expr, $entity:tt) => {
                    {
                        use $crate::namespace::entity_inner;
                        entity_inner!($mod_name, $entity_id, $entity)
                    }
                };
            }

            #[allow(unused)]
            pub(crate) use entity;

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

    use crate::{query::find, ufoid, TribleSet};

    use std::convert::TryInto;

    NS! {
        pub namespace knights {
            loves: "328edd7583de04e2bedd6bd4fd50e651" as crate::Id;
            name: "328147856cc1984f0806dbb824d2b4cb" as crate::types::ShortString;
            title: "328f2c33d2fdd675e733388770b2d6c4" as crate::types::ShortString;
        }
    }

    #[test]
    fn ns_entities() {
        let romeo = ufoid();
        let juliet = ufoid();

        knights::entity!(juliet, {
            name: "Juliet".try_into().unwrap(),
            loves: romeo,
            title: "Maiden".try_into().unwrap()
        });
        knights::entity!(romeo, {
            name: "Romeo".try_into().unwrap(),
            loves: juliet,
            title: "Prince".try_into().unwrap()
        });
        knights::entity!(
        {
            name: "Angelica".try_into().unwrap(),
            title: "Nurse".try_into().unwrap()
        });
    }


    #[test]
    fn ns_entity() {
        let juliet = ufoid();
        let romeo = ufoid();

        let mut tribles = TribleSet::new();
        tribles.union(&knights::entity!(juliet, {
            name: "Juliet".try_into().unwrap(),
            loves: romeo,
            title: "Maiden".try_into().unwrap()
        }));
        tribles.union(&knights::entity!(romeo, {
            name: "Romeo".try_into().unwrap(),
            loves: juliet,
            title: "Prince".try_into().unwrap()
        }));
        tribles.union(&knights::entity!({
            name: "Angelica".try_into().unwrap(),
            title: "Nurse".try_into().unwrap()
        }));
        println!("{:?}", tribles);
    }

    #[test]
    fn ns_pattern() {
        let juliet = ufoid();
        let romeo = ufoid();

        let mut kb = TribleSet::new();
        
        kb.union(&knights::entity!(juliet,
            {
                name: "Juliet".try_into().unwrap(),
                loves: romeo,
                title: "Maiden".try_into().unwrap()
            }));
        kb.union(&knights::entity!(romeo, {
            name: "Romeo".try_into().unwrap(),
            loves: juliet,
            title: "Prince".try_into().unwrap()
        }));
        kb.union(&knights::entity!({
            name: "Angelica".try_into().unwrap(),
            title: "Nurse".try_into().unwrap()
        }));

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
        let mut kb = TribleSet::new();
        (0..10000).for_each(|_| {
            let lover_a = ufoid();
            let lover_b = ufoid();
            kb.union(&knights::entity!(lover_a, {
                name: Name(EN).fake::<String>().try_into().unwrap(),
                loves: lover_b
            }));
            kb.union(&knights::entity!(lover_b, {
                name: Name(EN).fake::<String>().try_into().unwrap(),
                loves: lover_a
            }));
        });

        let juliet = ufoid();
        let romeo = ufoid();

        let mut data_kb = TribleSet::new();
        data_kb.union(&knights::entity!(juliet, {
            name: "Juliet".try_into().unwrap(),
            loves: romeo
        }));
        data_kb.union(&knights::entity!(romeo, {
            name: "Romeo".try_into().unwrap(),
            loves: juliet
        }));

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
