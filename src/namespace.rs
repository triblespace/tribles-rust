//! Namespaces give semantic meaning to the raw binary data stored in
//! [crate::prelude::TribleSet]s and [crate::prelude::BlobSet]s and provide a mapping from human readable
//! names to attribute ids and schemas.
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
    ($Namespace:path, $Set:expr, $EntityId:expr, {$($FieldName:ident : $Value:expr),* $(,)?}) => {
        {
            use $Namespace as ns;
            $({ let v: $crate::value::Value<ns::schemas::$FieldName> = $crate::value::ToValue::to_value($Value);
                $Set.insert(&$crate::trible::Trible::new(
                $EntityId,
                &ns::ids::$FieldName,
                &v));})*
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
            let a_var: $crate::query::Variable<$crate::value::schemas::genid::GenId> = $ctx.next_variable();
            let v_var: $crate::query::Variable<ns::schemas::$FieldName> = $ctx.next_variable();
            let v: $crate::value::Value<ns::schemas::$FieldName> = $Value.to_value();
            $constraints.push(Box::new(a_var.is(ns::ids::$FieldName.to_value())));
            $constraints.push(Box::new(v_var.is(v)));
            $constraints.push(Box::new($set.pattern($EntityId, a_var, v_var)));
        }

    };
    (@triple ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, $EntityId:ident, $FieldName:ident, $Value:expr)) => {
        {
            use $crate::query::TriblePattern;
            use $Namespace as ns;
            let a_var: $crate::query::Variable<$crate::value::schemas::genid::GenId> = $ctx.next_variable();
            let v_var: $crate::query::Variable<ns::schemas::$FieldName> = $Value;
            $constraints.push(Box::new(a_var.is(ns::ids::$FieldName.to_value())));
            $constraints.push(Box::new($set.pattern($EntityId, a_var, v_var)));
        }

    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {($EntityId:expr) @ $($FieldName:ident : $Value:tt),* $(,)?})) => {
        {
            let e_var: $crate::query::Variable<$crate::value::schemas::genid::GenId> = $ctx.next_variable();
            $constraints.push({ let e: $crate::id::RawId = $EntityId; Box::new(e_var.is(e.to_value()))});
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {$EntityId:ident @ $($FieldName:ident : $Value:tt),* $(,)?})) => {
        {
            let e_var: $crate::query::Variable<$crate::value::schemas::genid::GenId> = $EntityId;
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };

    (@entity ($constraints:ident, $ctx:ident, $set:ident, $Namespace:path, {$($FieldName:ident : $Value:tt),*})) => {
        {
            let e_var: $crate::query::Variable<$crate::value::schemas::genid::GenId> = $ctx.next_variable();
            $(pattern_inner!(@triple ($constraints, $ctx, $set, $Namespace, e_var, $FieldName, $Value));)*
        }
    };
    ($Namespace:path, $ctx:ident, $set:expr, [$($Entity:tt),*]) => {
        {
            let set = $set;
            let mut constraints: Vec<Box<dyn $crate::query::Constraint>> = vec!();
            $(pattern_inner!(@entity (constraints, $ctx, set, $Namespace, $Entity));)*
            $crate::query::intersectionconstraint::IntersectionConstraint::new(constraints)
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
///         "FF00FF00FF00FF00FF00FF00FF00FF00" as attr_name: tribles::value::schemas::genid::GenId;
///         "BBAABBAABBAABBAABBAABBAABBAABBAA" as attr_name2: tribles::value::schemas::shortstring::ShortString;
///     }
/// }
/// ```
///
/// will define a module with a structure similar to
///
/// ```
/// mod namespace_name {
///   use super::*; // enables lexical scoping
///   pub fn description() -> tribles::tribleset::TribleSet {
///     let set = tribles::tribleset::TribleSet::new();
///     // namespace described in terms of the `metadata` namespace
///     set    
///   }
///   pub mod ids {
///       use super::*;
///       use hex_literal::hex;
///       pub const attr_name: tribles::id::RawId  = hex!("FF00FF00FF00FF00FF00FF00FF00FF00");
///       pub const attr_name2: tribles::id::RawId  = hex!("BBAABBAABBAABBAABBAABBAABBAABBAA");
///   }
///   pub mod schemas {
///       use super::*;
///       pub use tribles::value::schemas::genid::GenId as attr_name;
///       pub use tribles::value::schemas::shortstring::ShortString as attr_name2;
///   }
/// }
/// ```
///
/// this allows you to access attribute ids and schemas via their human readable names, e.g.
/// `namespace_name::ids::attrName` and `namespace_name::schemas::attrName`.
#[macro_export]
macro_rules! NS {
    ($visibility:vis namespace $mod_name:ident {$($FieldId:literal as $FieldName:ident: $FieldType:ty;)*}) => {
        $visibility mod $mod_name {
            #![allow(unused)]
            use super::*;

            pub fn description() -> $crate::tribleset::TribleSet {
                use $crate::value::ValueSchema;

                let mut set = $crate::tribleset::TribleSet::new();
                $({let e = $crate::id::Id::new($crate::namespace::hex_literal::hex!($FieldId)).unwrap();
                   let value_schema_id = $crate::value::schemas::genid::GenId::to_value(<$FieldType as $crate::value::ValueSchema>::VALUE_SCHEMA_ID);
                   set.insert(&$crate::trible::Trible::new($crate::id::OwnedId::transmute_force(&e), &$crate::metadata::ATTR_ATTR_VALUE_SCHEMA, &value_schema_id));
                   if let Some(blob_schema_id) = <$FieldType as $crate::value::ValueSchema>::BLOB_SCHEMA_ID {
                      let blob_schema_id = $crate::value::schemas::genid::GenId::to_value(blob_schema_id);
                      set.insert(&$crate::trible::Trible::new($crate::id::OwnedId::transmute_force(&e), &$crate::metadata::ATTR_ATTR_BLOB_SCHEMA, &blob_schema_id));
                   }
                   let attr_name = $crate::value::schemas::shortstring::ShortString::to_value(stringify!($FieldName));
                   set.insert(&$crate::trible::Trible::new($crate::id::OwnedId::transmute_force(&e), &$crate::metadata::ATTR_LABEL, &attr_name));
                })*
                set
            }
            pub mod ids {
                #![allow(non_upper_case_globals, unused)]
                use super::*;
                $(pub const $FieldName:$crate::id::Id = $crate::id::Id::new($crate::namespace::hex_literal::hex!($FieldId)).unwrap();)*
            }
            pub mod schemas {
                #![allow(non_camel_case_types, unused)]
                use super::*;
                $(pub type $FieldName = $FieldType;)*
            }

            #[macro_pub::macro_pub]
            macro_rules! entity {
                ($entity:tt) => {
                    {
                        use $crate::namespace::entity_inner;
                        let mut set = $crate::tribleset::TribleSet::new();
                        let id: $crate::id::OwnedId = $crate::id::rngid();
                        entity_inner!($mod_name, &mut set, &id, $entity);
                        set
                    }
                };
                ($entity_id:expr, $entity:tt) => {
                    {
                        use $crate::namespace::entity_inner;
                        let mut set = $crate::tribleset::TribleSet::new();
                        let id: &$crate::id::OwnedId = $entity_id;
                        entity_inner!($mod_name, &mut set, id, $entity);
                        set
                    }
                };
            }

            #[macro_pub::macro_pub]
            macro_rules! pattern {
                ($set:expr, $pattern: tt) => {
                    {
                        use $crate::namespace::pattern_inner;
                        let ctx = __local_find_context!();
                        pattern_inner!($mod_name, ctx, $set, $pattern)
                    }
                };
            }
        }
    };
}

pub use NS;

#[cfg(test)]
mod tests {
    use fake::{faker::name::raw::Name, locales::EN, Fake};

    use crate::prelude::valueschemas::*;
    use crate::prelude::*;

    NS! {
        pub namespace knights4 {
            "328edd7583de04e2bedd6bd4fd50e651" as loves: GenId;
            "328147856cc1984f0806dbb824d2b4cb" as name: ShortString;
            "328f2c33d2fdd675e733388770b2d6c4" as title: ShortString;
        }
    }

    #[test]
    fn ns_entities() {
        let romeo = ufoid();
        let juliet = ufoid();

        knights4::entity!(&juliet, {
            name: "Juliet",
            loves: &romeo,
            title: "Maiden"
        });
        knights4::entity!(&romeo, {
            name: "Romeo",
            loves: &juliet,
            title: "Prince"
        });
        knights4::entity!(
        {
            name: "Angelica",
            title: "Nurse"
        });
    }

    #[test]
    fn ns_entity() {
        let juliet = ufoid();
        let romeo = ufoid();

        let mut tribles = TribleSet::new();
        tribles += knights4::entity!(&juliet, {
            name: "Juliet",
            loves: &romeo,
            title: "Maiden"
        });
        tribles += knights4::entity!(&romeo, {
            name: "Romeo",
            loves: &juliet,
            title: "Prince"
        });
        tribles += knights4::entity!({
            name: "Angelica",
            title: "Nurse"
        });
        println!("{:?}", tribles);
    }

    #[test]
    fn ns_pattern() {
        let juliet = ufoid();
        let romeo = ufoid();

        let mut kb = TribleSet::new();

        kb += knights4::entity!(&juliet,
        {
            name: "Juliet",
            loves: &romeo,
            title: "Maiden"
        });
        kb += knights4::entity!(&romeo, {
            name: "Romeo",
            loves: &juliet,
            title: "Prince"
        });
        kb += knights4::entity!({
            name: "Angelica",
            title: "Nurse"
        });

        let r: Vec<_> = find!(
            (juliet, name),
            knights4::pattern!(&kb, [
            {name: ("Romeo"),
             loves: juliet},
            {juliet @
                name: name
            }])
        )
        .collect();
        assert_eq!(vec![(juliet.to_value(), "Juliet".to_value(),)], r);
    }

    #[test]
    fn ns_pattern_large() {
        let mut kb = TribleSet::new();
        (0..10000).for_each(|_| {
            let lover_a = ufoid();
            let lover_b = ufoid();
            kb += knights4::entity!(&lover_a, {
                name: Name(EN).fake::<String>(),
                loves: &lover_b
            });
            kb += knights4::entity!(&lover_b, {
                name: Name(EN).fake::<String>(),
                loves: &lover_a
            });
        });

        let juliet = ufoid();
        let romeo = ufoid();

        let mut data_kb = TribleSet::new();
        data_kb += knights4::entity!(&juliet, {
            name: "Juliet",
            loves: &romeo
        });
        data_kb += knights4::entity!(&romeo, {
            name: "Romeo",
            loves: &juliet
        });

        kb += data_kb;

        let r: Vec<_> = find!(
            (juliet, name),
            knights4::pattern!(&kb, [
            {name: ("Romeo"),
            loves: juliet},
            {juliet @
                name: name
            }])
        )
        .collect();

        assert_eq!(vec![(juliet.to_value(), "Juliet".to_value(),)], r);
    }
}
