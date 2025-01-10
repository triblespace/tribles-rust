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

/// Helper macro for constructing trible entries for an entity.
/// Hidden by default, used internally by the `entity!` macro.
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

/// Helper macro for constructing pattern-based queries.
/// Hidden by default, used internally by the `pattern!` macro.
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
            $constraints.push({ let e: $crate::id::Id = $EntityId; Box::new(e_var.is(e.to_value()))});
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
    ($Namespace:path, $ctx:ident, $set:ident, [$($Entity:tt),*]) => {
        {
            let mut constraints: Vec<Box<dyn $crate::query::Constraint>> = vec!();
            $(pattern_inner!(@entity (constraints, $ctx, $set, $Namespace, $Entity));)*
            $crate::query::intersectionconstraint::IntersectionConstraint::new(constraints)
        }
    };
}

pub use pattern_inner;

pub use hex_literal;

/// Defines a Rust module to represent a namespace, along with convenience macros.
/// The `namespace` block maps human-readable names to attribute IDs and type schemas.
#[macro_export]
macro_rules! NS {
    ($(#[doc = $ns_doc:literal])* $visibility:vis namespace $mod_name:ident {$($(#[doc = $field_doc:literal])* $FieldId:literal as $FieldName:ident: $FieldType:ty;)*}) => {
        $(#[doc=$ns_doc])*
        $visibility mod $mod_name {
            #![allow(unused)]
            use super::*;

            pub fn description() -> $crate::trible::TribleSet {
                use $crate::value::ValueSchema;

                let mut set = $crate::trible::TribleSet::new();
                $({let e = $crate::id::Id::new($crate::namespace::hex_literal::hex!($FieldId)).unwrap();
                   let value_schema_id = $crate::value::schemas::genid::GenId::value_from(<$FieldType as $crate::value::ValueSchema>::VALUE_SCHEMA_ID);
                   set.insert(&$crate::trible::Trible::force(&e, &$crate::metadata::ATTR_VALUE_SCHEMA, &value_schema_id));
                   if let Some(blob_schema_id) = <$FieldType as $crate::value::ValueSchema>::BLOB_SCHEMA_ID {
                      let blob_schema_id = $crate::value::schemas::genid::GenId::value_from(blob_schema_id);
                      set.insert(&$crate::trible::Trible::force(&e, &$crate::metadata::ATTR_BLOB_SCHEMA, &blob_schema_id));
                   }
                   let attr_name = $crate::value::schemas::shortstring::ShortString::value_from(stringify!($FieldName));
                   set.insert(&$crate::trible::Trible::force(&e, &$crate::metadata::ATTR_NAME, &attr_name));
                })*
                set
            }
            pub mod ids {
                #![allow(non_upper_case_globals, unused)]
                use super::*;
                $($(#[doc = $field_doc])* pub const $FieldName:$crate::id::Id = $crate::id::Id::new($crate::namespace::hex_literal::hex!($FieldId)).unwrap();)*
            }
            pub mod schemas {
                #![allow(non_camel_case_types, unused)]
                use super::*;
                $($(#[doc = $field_doc])* pub type $FieldName = $FieldType;)*
            }

            #[macro_pub::macro_pub]
            macro_rules! entity {
                ($entity:tt) => {
                    {
                        use $crate::namespace::entity_inner;
                        let mut set = $crate::trible::TribleSet::new();
                        let id: $crate::id::ExclusiveId = $crate::id::rngid();
                        entity_inner!($mod_name, &mut set, &id, $entity);
                        set
                    }
                };
                ($entity_id:expr, $entity:tt) => {
                    {
                        use $crate::namespace::entity_inner;
                        let mut set = $crate::trible::TribleSet::new();
                        let id: &$crate::id::ExclusiveId = $entity_id;
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
                        let set = $set;
                        pattern_inner!($mod_name, ctx, set, $pattern)
                    }
                };
            }
        }
    };
}

pub use NS;

#[cfg(test)]
mod tests {
    use crate::examples::literature;
    use crate::prelude::*;

    use fake::{faker::name::raw::Name, locales::EN, Fake};

    #[test]
    fn ns_entity() {
        let mut tribles = TribleSet::new();

        let author = ufoid();
        let book = ufoid();

        tribles += literature::entity!(&author, {
            firstname: "William",
            lastname: "Shakespeare",
        });

        tribles += literature::entity!(&book, {
            title: "Hamlet",
            author: &author,
            quote: "To be, or not to be, that is the question.".to_blob().as_handle()
        });

        assert_eq!(tribles.len(), 5);
    }

    #[test]
    fn ns_pattern() {
        let author = ufoid();
        let book = ufoid();

        let mut kb = TribleSet::new();

        kb += literature::entity!(&author, {
            firstname: "William",
            lastname: "Shakespeare",
        });
        kb += literature::entity!(&book, {
            title: "Hamlet",
            author: &author,
            quote: "To be, or not to be, that is the question.".to_blob().as_handle()
        });

        let r: Vec<_> = find!(
            (book, title, firstname),
            literature::pattern!(&kb, [
            {firstname: firstname,
             lastname: ("Shakespeare")},
            {book @
                title: title,
                author: (author)
            }])
        )
        .collect();
        assert_eq!(
            vec![(book.to_value(), "Hamlet".to_value(), "William".to_value())],
            r
        );
    }

    #[test]
    fn ns_pattern_large() {
        let mut kb = TribleSet::new();
        (0..10000).for_each(|_| {
            let author = ufoid();
            let book = ufoid();
            kb += literature::entity!(&author, {
                firstname: Name(EN).fake::<String>(),
                lastname: Name(EN).fake::<String>()
            });
            kb += literature::entity!(&book, {
                title: Name(EN).fake::<String>(),
                author: &author
            });
        });

        let shakespeare = ufoid();
        let hamlet = ufoid();

        let mut data_kb = TribleSet::new();
        data_kb += literature::entity!(&shakespeare, {
            firstname: "William",
            lastname: "Shakespeare"
        });
        data_kb += literature::entity!(&hamlet, {
            title: "Hamlet",
            author: &shakespeare,
            quote: "To be, or not to be, that is the question.".to_blob().as_handle()
        });

        kb += data_kb;

        let r: Vec<_> = find!(
            (author, hamlet, title),
            literature::pattern!(&kb, [
            {author @
             firstname: ("William"),
             lastname: ("Shakespeare")},
            {hamlet @
                title: title,
                author: author
            }])
        )
        .collect();

        assert_eq!(
            vec![(
                shakespeare.to_value(),
                hamlet.to_value(),
                "Hamlet".to_value(),
            )],
            r
        );
    }
}
