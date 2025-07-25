#![cfg(kani)]

use crate::prelude::*;
use crate::value::schemas::{genid::GenId, UnknownValue};

NS! {
    /// Namespace used by the query harness with unconstrained values.
    pub namespace qns {
        "A74AA63539354CDA47F387A4C3A8D54C" as title: UnknownValue;
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        "0DBB530B37B966D137C50B943700EDB2" as firstname: UnknownValue;
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: UnknownValue;
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn query_harness() {
    // Build a small knowledge base with one author and one book.
    let author_raw: [u8; 16] = kani::any();
    let book_raw: [u8; 16] = kani::any();
    kani::assume(author_raw != [0u8; 16]);
    kani::assume(book_raw != [0u8; 16]);
    kani::assume(book_raw != author_raw);
    let author = ExclusiveId::force(Id::new(author_raw).unwrap());
    let book = ExclusiveId::force(Id::new(book_raw).unwrap());

    let firstname_raw: [u8; 32] = kani::any();
    let lastname_raw: [u8; 32] = kani::any();
    let title_raw: [u8; 32] = kani::any();

    let firstname = Value::<UnknownValue>::new(firstname_raw);
    let lastname = Value::<UnknownValue>::new(lastname_raw);
    let title = Value::<UnknownValue>::new(title_raw);

    let mut set = TribleSet::new();
    set += qns::entity!(&author, {
        firstname: firstname,
        lastname: lastname,
    });
    set += qns::entity!(&book, {
        title: title,
        author: &author,
    });

    // Find the title and author first name for Shakespeare's book.
    let result: Vec<_> = find!(
        (book, title, firstname),
        qns::pattern!(&set, [
            { firstname: firstname,
              lastname: (lastname) },
            { book @
                title: title,
                author: (author) }
        ])
    )
    .collect();

    assert_eq!(vec![(book.to_value(), title, firstname)], result);
}
