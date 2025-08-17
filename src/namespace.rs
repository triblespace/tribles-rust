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
        ::tribles_macros::namespace!(::tribles, $($tt)*);
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

        tribles += literature::entity!(&author, {
            firstname: "William",
            lastname: "Shakespeare",
        });

        tribles += literature::entity!(&book, {
            title: "Hamlet",
            author: &author,
            quote: "To be, or not to be, that is the question.".to_blob().get_handle()
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
            quote: "To be, or not to be, that is the question.".to_blob().get_handle()
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
        (0..100).for_each(|_| {
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
            quote: "To be, or not to be, that is the question.".to_blob().get_handle()
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

    #[test]
    fn ns_delta() {
        let mut base = TribleSet::new();
        (0..10).for_each(|_| {
            let a = ufoid();
            let b = ufoid();
            base += literature::entity!(&a, {
                firstname: Name(EN).fake::<String>(),
                lastname: Name(EN).fake::<String>()
            });
            base += literature::entity!(&b, {
                title: Name(EN).fake::<String>(),
                author: &a
            });
        });

        let mut updated = base.clone();
        let shakespeare = ufoid();
        let hamlet = ufoid();
        updated += literature::entity!(&shakespeare, {
            firstname: "William",
            lastname: "Shakespeare"
        });
        updated += literature::entity!(&hamlet, {
            title: "Hamlet",
            author: &shakespeare,
            quote: "To be, or not to be, that is the question.".to_blob().get_handle()
        });

        let delta = &updated.difference(&base);
        let r: Vec<_> = find!(
            (author, hamlet, title),
            literature::pattern_changes!(&updated, delta, [
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
