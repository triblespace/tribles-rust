#![cfg(kani)]

use crate::examples::literature;
use crate::prelude::*;

#[kani::proof]
#[kani::unwind(5)]
fn query_macro_harness() {
    // Build a small knowledge base with one author and one book.
    let author = ufoid();
    let book = ufoid();

    let mut set = TribleSet::new();
    set += literature::entity!(&author, {
        firstname: "William",
        lastname: "Shakespeare",
    });
    set += literature::entity!(&book, {
        title: "Hamlet",
        author: &author,
    });

    // Find the title and author first name for Shakespeare's book.
    let result: Vec<_> = find!(
        (book, title, firstname),
        literature::pattern!(&set, [
            { firstname: firstname,
              lastname: ("Shakespeare") },
            { book @
                title: title,
                author: (author) }
        ])
    )
    .collect();

    assert_eq!(
        vec![(book.to_value(), "Hamlet".to_value(), "William".to_value()),],
        result
    );
}
