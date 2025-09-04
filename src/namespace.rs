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
pub use hex_literal;

/// Defines a Rust module to represent a namespace, along with convenience macros.
/// The `namespace` block maps human-readable names to attribute IDs and type schemas.
#[macro_export]
macro_rules! NS {
    ($($tt:tt)*) => {
        ::tribles::macros::namespace!(::tribles, $($tt)*);
    };
}

pub use NS;

#[cfg(test)]
mod tests {
    use crate::examples::literature;
    use crate::prelude::*;

    use fake::faker::name::raw::Name;
    use fake::locales::EN;
    use fake::Fake;

    #[test]
    fn ns_entity() {
        let mut tribles = TribleSet::new();

        let author = ufoid();
        let book = ufoid();

        tribles += crate::entity!(&author, {
            literature::firstname: "William",
            literature::lastname: "Shakespeare",
        });

        tribles += crate::entity!(&book, {
            literature::title: "Hamlet",
            literature::author: &author,
            literature::quote: "To be, or not to be, that is the question.".to_blob().get_handle()
        });

        assert_eq!(tribles.len(), 5);
    }

    #[test]
    fn ns_pattern() {
        let author = ufoid();
        let book = ufoid();

        let mut kb = TribleSet::new();

        kb += crate::entity!(&author, {
            literature::firstname: "William",
            literature::lastname: "Shakespeare",
        });
        kb += crate::entity!(&book, {
            literature::title: "Hamlet",
            literature::author: &author,
            literature::quote: "To be, or not to be, that is the question.".to_blob().get_handle()
        });

        let r: Vec<_> = find!(
            (book, title, firstname),
            crate::pattern!(&kb, [
            {literature::firstname: firstname,
             literature::lastname: ("Shakespeare")},
            {book @
                literature::title: title,
                literature::author: (author)
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
        (0..100).for_each(|_| {
            let author = ufoid();
            let book = ufoid();
            kb += crate::entity!(&author, {
                literature::firstname: Name(EN).fake::<String>(),
                literature::lastname: Name(EN).fake::<String>()
            });
            kb += crate::entity!(&book, {
                literature::title: Name(EN).fake::<String>(),
                literature::author: &author
            });
        });

        let shakespeare = ufoid();
        let hamlet = ufoid();

        let mut data_kb = TribleSet::new();
        data_kb += crate::entity!(&shakespeare, {
            literature::firstname: "William",
            literature::lastname: "Shakespeare"
        });
        data_kb += crate::entity!(&hamlet, {
            literature::title: "Hamlet",
            literature::author: &shakespeare,
            literature::quote: "To be, or not to be, that is the question.".to_blob().get_handle()
        });

        kb += data_kb;

        let r: Vec<_> = find!(
            (author, hamlet, title),
            crate::pattern!(&kb, [
            {author @
             literature::firstname: ("William"),
             literature::lastname: ("Shakespeare")},
            {hamlet @
                literature::title: title,
                literature::author: author
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

    #[test]
    fn ns_delta() {
        let mut base = TribleSet::new();
        (0..10).for_each(|_| {
            let a = ufoid();
            let b = ufoid();
            base += crate::entity!(&a, {
                literature::firstname: Name(EN).fake::<String>(),
                literature::lastname: Name(EN).fake::<String>()
            });
            base += crate::entity!(&b, {
                literature::title: Name(EN).fake::<String>(),
                literature::author: &a
            });
        });

        let mut updated = base.clone();
        let shakespeare = ufoid();
        let hamlet = ufoid();
        updated += crate::entity!(&shakespeare, {
            literature::firstname: "William",
            literature::lastname: "Shakespeare"
        });
        updated += crate::entity!(&hamlet, {
            literature::title: "Hamlet",
            literature::author: &shakespeare,
            literature::quote: "To be, or not to be, that is the question.".to_blob().get_handle()
        });

        let delta = &updated.difference(&base);
        let r: Vec<_> = find!(
            (author, hamlet, title),
            crate::pattern_changes!(&updated, delta, [
            {author @
             literature::firstname: ("William"),
             literature::lastname: ("Shakespeare")},
            {hamlet @
                literature::title: title,
                literature::author: author
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
