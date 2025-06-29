#![cfg(kani)]

use crate::examples::literature;
use crate::prelude::*;
use kani::BoundedArbitrary;

#[kani::proof]
#[kani::unwind(5)]
fn query_macro_harness() {
    // Build a small knowledge base with one author and one book.
    let author = ExclusiveId::force(Id::new([1u8; 16]).unwrap());
    let book = ExclusiveId::force(Id::new([2u8; 16]).unwrap());

    let firstname_str = String::bounded_any::<32>();
    let lastname_str = String::bounded_any::<32>();
    let title_str = String::bounded_any::<32>();

    let mut set = TribleSet::new();
    set += literature::entity!(&author, {
        firstname: &firstname_str,
        lastname: &lastname_str,
    });
    set += literature::entity!(&book, {
        title: &title_str,
        author: &author,
    });

    // Find the title and author first name for Shakespeare's book.
    let result: Vec<_> = find!(
        (book, title, firstname),
        literature::pattern!(&set, [
            { firstname: firstname,
              lastname: (&lastname_str) },
            { book @
                title: title,
                author: (author) }
        ])
    )
    .collect();

    assert_eq!(
        vec![(
            book.to_value(),
            title_str.to_value(),
            firstname_str.to_value()
        )],
        result
    );
}
