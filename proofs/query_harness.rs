#![cfg(kani)]

use crate::examples::literature;
use crate::prelude::*;
use kani::BoundedArbitrary;

#[kani::proof]
#[kani::unwind(5)]
fn query_harness() {
    // Build a small knowledge base with one author and one book.
    let author_raw: [u8; 16] = kani::any();
    let book_raw: [u8; 16] = kani::any();
    kani::assume(author_raw != [0u8; 16]);
    kani::assume(book_raw != [0u8; 16]);
    kani::assume(book_raw != author_raw);
    let author = ExclusiveId::force(Id::new(author_raw).unwrap());
    let book = ExclusiveId::force(Id::new(book_raw).unwrap());

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

/// Test generated for harness `proofs::query_harness::query_harness`
///
/// Check for `unsupported_construct`: "ptr_mask is not currently supported by Kani. Please post your example at https://github.com/model-checking/kani/issues/new/choose"

#[test]
fn kani_concrete_playback_query_harness_18094746763674326969() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 127
        vec![127],
        // 127
        vec![127],
        // 127
        vec![127],
        // 127
        vec![127],
        // 127
        vec![127],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 0
        vec![0],
        // 128
        vec![128],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 0
        vec![0],
        // 128
        vec![128],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 0
        vec![0],
        // 128
        vec![128],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 192
        vec![192],
        // 0
        vec![0],
        // 192
        vec![192],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
        // 255
        vec![255],
    ];
    kani::concrete_playback_run(concrete_vals, query_harness);
}
