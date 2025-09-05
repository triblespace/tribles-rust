use tribles::prelude::valueschemas::*;
use tribles::prelude::*;
use crate::pattern;
use crate::entity;
use crate::pattern_changes;
use crate::path;


NS! {
    pub namespace literature {
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
        "A74AA63539354CDA47F387A4C3A8D54C" as title: ShortString;
    }
}

#[test]
fn pattern_changes_finds_new_inserts() {
    let base = TribleSet::new();

    let mut updated = base.clone();
    let shakespeare = ufoid();
    let hamlet = ufoid();
    updated += entity!(&shakespeare, { literature::firstname: "William", literature::lastname: "Shakespeare" });
    updated += entity!(&hamlet, { literature::title: "Hamlet", literature::author: &shakespeare });

    let delta = updated.difference(&base);

    let results: Vec<_> = find!(
        (author: Value<_>, book: Value<_>, title: Value<_>),
        literature::pattern_changes!(&updated, &delta, [
            { author @ firstname: ("William"), lastname: ("Shakespeare") },
            { book @ author: author, title: title }
        ])
    )
    .collect();

    assert_eq!(
        results,
        vec![(
            shakespeare.to_value(),
            hamlet.to_value(),
            "Hamlet".to_value(),
        )]
    );
}

#[test]
fn pattern_changes_empty_delta_returns_no_matches() {
    let mut kb = TribleSet::new();
    let shakespeare = ufoid();
    kb += entity!(&shakespeare, { literature::firstname: "William", literature::lastname: "Shakespeare" });

    let delta = TribleSet::new();

    let results: Vec<_> = find!(
        (a: Value<_>),
        literature::pattern_changes!(&kb, &delta, [
            { a @ lastname: ("Shakespeare") }
        ])
    )
    .collect();

    assert!(results.is_empty());
}