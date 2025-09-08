use crate::entity;
use crate::path;
use crate::pattern;
use crate::pattern_changes;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::valueschemas::*;
use tribles::prelude::*;
use tribles::repo::memoryrepo::MemoryRepo;
use tribles::repo::Repository;

pub mod literature {
    #![allow(unused)]
    use super::*;
    crate::fields! {
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
        "A74AA63539354CDA47F387A4C3A8D54C" as title: ShortString;
    }
}

fn main() {
    // ANCHOR: pattern_changes_example
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let mut ws = repo.branch("main").expect("branch");

    // Commit initial data
    let shakespeare = ufoid();
    let hamlet = ufoid();
    let mut base = TribleSet::new();
    base += entity!(&shakespeare, { literature::firstname: "William", literature::lastname: "Shakespeare" });
    base += entity!(&hamlet, { literature::title: "Hamlet", literature::author: &shakespeare });
    ws.commit(base.clone(), None);
    let c1 = ws.head().unwrap();

    // Commit a new book
    let macbeth = ufoid();
    let mut change = TribleSet::new();
    change += entity!(&macbeth, { literature::title: "Macbeth", literature::author: &shakespeare });
    ws.commit(change.clone(), None);
    let c2 = ws.head().unwrap();

    // Compute updated state and delta between commits
    let base_state = ws.checkout(c1).expect("base");
    let updated = ws.checkout(c2).expect("updated");
    let delta = updated.difference(&base_state);

    // Find new titles by Shakespeare
    let results: Vec<_> = find!(
        (author: Value<_>, book: Value<_>, title: Value<_>),
        pattern_changes!(&updated, &delta, [
            { author @ literature::firstname: ("William"), literature::lastname: ("Shakespeare") },
            { book @ literature::author: author, literature::title: title }
        ])
    )
    .map(|(_, b, t)| (b, t))
    .collect();

    println!("{results:?}");
    // ANCHOR_END: pattern_changes_example
}
